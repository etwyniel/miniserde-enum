use crate::attr;
use crate::bound;
use crate::TagType;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote, DataEnum, DeriveInput, Fields, FieldsNamed, FieldsUnnamed, Ident, Result};

pub fn derive(input: &DeriveInput, enumeration: &DataEnum) -> Result<TokenStream> {
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let ident = &input.ident;
    let tag_type = attr::tag_type(&input.attrs, &enumeration)?;
    let names = enumeration
        .variants
        .iter()
        .map(attr::name_of_variant)
        .collect::<Result<Vec<_>>>()?;
    let begin = enumeration
        .variants
        .iter()
        .zip(names.iter())
        .map(|(variant, name)| {
            let var_ident = &variant.ident;
            Ok(match &variant.fields {
                Fields::Unit => {
                    let implementation = serialize_unit(name, &tag_type)?;
                    quote! {
                        #ident::#var_ident => {#implementation}
                    }
                }
                Fields::Named(fields) => {
                    let implementation = serialize_named(input, &fields, name, &tag_type)?;
                    let field_ident = fields
                        .named
                        .iter()
                        .map(|field| &field.ident)
                        .collect::<Vec<_>>();

                    quote! {
                        #ident::#var_ident{#(#field_ident),*} => {
                            #implementation
                        }
                    }
                }
                Fields::Unnamed(fields) => {
                    let field_ident = (0..fields.unnamed.len())
                        .map(|i| format!("f{}", i))
                        .map(|id| Ident::new(&id, Span::call_site()))
                        .collect::<Vec<_>>();
                    let implementation =
                        serialize_unnamed(input, fields, &field_ident, name, &tag_type)?;
                    quote! {
                        #ident::#var_ident(#(#field_ident),*) => {
                            #implementation
                        }
                    }
                }
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        const _: () = {
            impl #impl_generics miniserde::Serialize for #ident #ty_generics #where_clause {
                fn begin(&self) -> miniserde::ser::Fragment {
                    match self {
                        #(#begin)*
                    }
                }
            }
        };
    })
}

fn serialize_unit(variant_name: &str, tag_type: &TagType) -> Result<TokenStream> {
    Ok(if let TagType::Internal(tag) = &tag_type {
        quote! {
            struct __Map {
                state: miniserde::export::usize,
            }

            impl miniserde::ser::Map for __Map {
                fn next(&mut self) -> miniserde::export::Option<(miniserde::export::Cow<miniserde::export::str>, &dyn miniserde::Serialize)> {
                    let __state = self.state;
                    self.state = __state + 1;
                    match __state {
                        0 => miniserde::export::Some((
                            miniserde::export::Cow::Borrowed(#tag),
                            &#variant_name,
                        )),
                        _ => miniserde::export::None,
                    }
                }
            }

            miniserde::ser::Fragment::Map(miniserde::export::Box::new(__Map {state: 0}))
        }
    } else {
        quote! {miniserde::ser::Fragment::Str(miniserde::export::Cow::Borrowed(#variant_name))}
    })
}

fn serialize_named(
    input: &DeriveInput,
    fields: &FieldsNamed,
    variant_name: &str,
    tag_type: &TagType,
) -> Result<TokenStream> {
    let field_ident = fields
        .named
        .iter()
        .map(|field| &field.ident)
        .collect::<Vec<_>>();
    let field_name = fields
        .named
        .iter()
        .map(attr::name_of_field)
        .collect::<Result<Vec<_>>>()?;
    let field_type = fields
        .named
        .iter()
        .map(|field| &field.ty)
        .collect::<Vec<_>>();
    let (_, _, where_clause) = input.generics.split_for_impl();
    let wrapper_generics = bound::with_lifetime_bound(&input.generics, "'__b");
    let (wrapper_impl_generics, wrapper_ty_generics, _) = wrapper_generics.split_for_impl();
    let bound = parse_quote!(miniserde::Serialize);
    let bounded_where_clause = bound::where_clause_with_bound(&input.generics, bound);
    let cow = quote!(miniserde::export::Cow);
    let some = quote!(miniserde::export::Some);
    if let TagType::External = tag_type {
        Ok(quote! {
            use miniserde::Serialize;
            #[derive(Serialize)]
            struct __AsStruct #wrapper_impl_generics #where_clause {
                #(#field_ident: &'__b #field_type),*,
            }

            struct __SuperMap #wrapper_impl_generics #where_clause {
                data: __AsStruct #wrapper_ty_generics,
                state: miniserde::export::usize,
            }

            impl #wrapper_impl_generics miniserde::ser::Map for __SuperMap #wrapper_ty_generics #bounded_where_clause {
                fn next(&mut self) -> miniserde::export::Option<(#cow<miniserde::export::str>, &dyn miniserde::Serialize)> {
                    let __state = self.state;
                    self.state = __state + 1;
                    match __state {
                        0 => #some((#cow::Borrowed(#variant_name), &self.data)),
                        _ => miniserde::export::None,
                    }
                }
            }

            miniserde::ser::Fragment::Map(miniserde::export::Box::new(__SuperMap {
                data: __AsStruct { #(#field_ident),* },
                state: 0,
            }))
        })
    } else {
        let (start, tag_arm) = if let TagType::Internal(ref tag) = &tag_type {
            (
                0,
                quote! {0 => #some((#cow::Borrowed(#tag), &#variant_name)),},
            )
        } else {
            (1usize, quote!())
        };
        let index = 1usize..;
        Ok(quote! {
            struct __Map #wrapper_impl_generics {
                #(#field_ident: &'__b #field_type),*,
                state: miniserde::export::usize,
            }

            impl #wrapper_impl_generics miniserde::ser::Map for __Map #wrapper_ty_generics #where_clause {
                fn next(&mut self) -> miniserde::export::Option<(#cow<miniserde::export::str>, &dyn miniserde::Serialize)> {
                    let __state = self.state;
                    self.state = __state + 1;
                    match __state {
                        #tag_arm
                        #(#index => {
                            #some((
                                    #cow::Borrowed(#field_name),
                                    self.#field_ident,
                                    ))
                        })*,
                        _ => miniserde::export::None,
                    }
                }
            }

            miniserde::ser::Fragment::Map(miniserde::export::Box::new(__Map {
                #(#field_ident),*,
                state: #start,
            }))
        })
    }
}

fn serialize_unnamed(
    input: &DeriveInput,
    fields: &FieldsUnnamed,
    field_ident: &[Ident],
    variant_name: &str,
    tag_type: &TagType,
) -> Result<TokenStream> {
    let field_type = fields
        .unnamed
        .iter()
        .map(|field| &field.ty)
        .collect::<Vec<_>>();
    let (_, _, where_clause) = input.generics.split_for_impl();
    let wrapper_generics = bound::with_lifetime_bound(&input.generics, "'__b");
    let (wrapper_impl_generics, wrapper_ty_generics, _) = wrapper_generics.split_for_impl();
    let bound = parse_quote!(miniserde::Serialize);
    let bounded_where_clause = bound::where_clause_with_bound(&input.generics, bound);
    let index = 0usize..;
    let ex = quote!(miniserde::export);
    let seq = if field_ident.len() == 1 {
        quote! { #(#field_ident.begin())* }
    } else {
        quote! {
            struct __Seq #wrapper_impl_generics #where_clause {
                #(#field_ident: &'__b #field_type),*,
                state: miniserde::export::usize,
            }

            impl #wrapper_impl_generics miniserde::ser::Seq for __Seq #wrapper_ty_generics #bounded_where_clause {
                fn next(&mut self) -> #ex::Option<&dyn miniserde::Serialize> {
                    let __state = self.state;
                    self.state = __state + 1;
                    match __state {
                        #(#index => #ex::Some(self.#field_ident)),*,
                        _ => #ex::None,
                    }
                }
            }

            miniserde::ser::Fragment::Seq(#ex::Box::new(__Seq {
                #(#field_ident),*,
                state: 0,
            }))
        }
    };
    Ok(if let TagType::External = tag_type {
        quote! {
            struct __AsStruct #wrapper_impl_generics (#(&'__b #field_type),*) #where_clause;

            impl #wrapper_impl_generics miniserde::Serialize for __AsStruct #wrapper_ty_generics #bounded_where_clause {
                fn begin(&self) -> miniserde::ser::Fragment {
                    let __AsStruct(#(#field_ident),*) = self;
                    #seq
                }
            }

            struct __SuperMap #wrapper_impl_generics #where_clause {
                data: __AsStruct #wrapper_ty_generics,
                state: bool,
            }

            impl #wrapper_impl_generics miniserde::ser::Map for __SuperMap #wrapper_ty_generics #bounded_where_clause {
                fn next(&mut self) -> miniserde::export::Option<(#ex::Cow<miniserde::export::str>, &dyn miniserde::Serialize)> {
                    if self.state {
                        return miniserde::export::None;
                    }
                    self.state = true;
                    #ex::Some((#ex::Cow::Borrowed(#variant_name), &self.data))
                }
            }

            miniserde::ser::Fragment::Map(#ex::Box::new(__SuperMap {
                data: __AsStruct ( #(#field_ident),* ),
                state: false,
            }))
        }
    } else {
        quote! {
            #seq
        }
    })
}
