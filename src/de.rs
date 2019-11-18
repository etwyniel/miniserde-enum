use crate::attr;
use crate::bound;
use crate::TagType;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse_quote, DataEnum, DeriveInput, Error, Fields, FieldsNamed, FieldsUnnamed, Ident, Result,
    Variant,
};

pub fn derive(input: &DeriveInput, enumeration: &DataEnum) -> Result<TokenStream> {
    let tag_type = attr::tag_type(&input.attrs, &enumeration)?;
    match &tag_type {
        TagType::External => deserialize_external(input, enumeration),
        _ => Err(Error::new(
            Span::call_site(),
            "Only externally tagged enums are supported",
        )),
    }
}

pub fn deserialize_external(input: &DeriveInput, enumeration: &DataEnum) -> Result<TokenStream> {
    let ident = &input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let wrapper_generics = bound::with_lifetime_bound(&input.generics, "'__a");
    let (wrapper_impl_generics, wrapper_ty_generics, _) = wrapper_generics.split_for_impl();
    let bound = parse_quote!(miniserde::Deserialize);
    let bounded_where_clause = bound::where_clause_with_bound(&input.generics, bound);

    let (unit_variants, struct_variants): (Vec<_>, Vec<_>) =
        enumeration.variants.iter().partition(|v| {
            if let Fields::Unit = &v.fields {
                true
            } else {
                false
            }
        });
    let struct_names = struct_variants
        .iter()
        .map(|variant| {
            Ident::new(
                &format!("__{}_{}_Struct", ident, variant.ident),
                Span::call_site(),
            )
        })
        .collect::<Vec<_>>();
    let struct_variant_names = struct_variants
        .iter()
        .cloned()
        .map(attr::name_of_variant)
        .collect::<Result<Vec<_>>>()?;
    let struct_variant_ident = struct_variants
        .iter()
        .map(|variant| &variant.ident)
        .collect::<Vec<_>>();
    let structs = struct_variants
        .iter()
        .zip(struct_names.iter())
        .map(|(variant, ident)| variant_as_struct(variant, ident, &input.ident))
        .collect::<Result<Vec<_>>>()?;
    let unit_variant_idents = unit_variants.iter().map(|v| &v.ident).collect::<Vec<_>>();
    let unit_variant_names = unit_variants
        .iter()
        .cloned()
        .map(attr::name_of_variant)
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        const _: () = {
            struct __Visitor #impl_generics #where_clause {
                __out: miniserde::export::Option<#ident #ty_generics>,
            }

            impl #impl_generics miniserde::Deserialize for #ident #ty_generics #bounded_where_clause {
                fn begin(__out: &mut miniserde::export::Option<Self>) -> &mut dyn miniserde::de::Visitor {
                    unsafe {
                        &mut *{
                            __out
                                as *mut miniserde::export::Option<Self>
                                as *mut __Visitor #ty_generics
                        }
                    }
                }
            }

            impl #impl_generics miniserde::de::Visitor for __Visitor #ty_generics #bounded_where_clause {
                fn map(&mut self) -> miniserde::Result<miniserde::export::Box<dyn miniserde::de::Map + '_>> {
                    Ok(miniserde::export::Box::new(__State{
                        __out: &mut self.__out,
                        #(#struct_variant_ident: None,)*
                    }))
                }

                fn string(&mut self, s: &str) -> miniserde::Result<()> {
                    match s {
                        #(#unit_variant_names => {
                            self.__out = Some(#ident::#unit_variant_idents);
                            Ok(())
                        })*
                        _ => Err(miniserde::Error)
                    }
                }
            }

            #[allow(non_snake_case)]
            struct __State #wrapper_impl_generics #where_clause {
                #(#[allow(non_snake_case)] #struct_variant_ident: miniserde::export::Option<#struct_names>,)*
                __out: &'__a mut miniserde::export::Option<#ident #ty_generics>,
            }

            #(#structs)*

            impl #wrapper_impl_generics miniserde::de::Map for __State #wrapper_ty_generics #bounded_where_clause {
                fn key(&mut self, k: &miniserde::export::str) -> miniserde::Result<&mut dyn miniserde::de::Visitor> {
                    match k {
                        #(
                            #struct_variant_names => miniserde::export::Ok(#struct_names::begin(&mut self.#struct_variant_ident)),
                        )*
                            _ => miniserde::export::Ok(miniserde::de::Visitor::ignore()),
                    }
                }

                fn finish(&mut self) -> miniserde::Result<()> {
                    #(
                        if let Some(val) = self.#struct_variant_ident.take() {
                            *self.__out = miniserde::export::Some(val.as_enum());
                            return miniserde::export::Ok(());
                        }
                    )*
                    miniserde::export::Err(miniserde::Error)
                }
            }
        };
    })
}

pub fn variant_as_struct(
    variant: &Variant,
    ident: &Ident,
    enum_ident: &Ident,
) -> Result<TokenStream> {
    match &variant.fields {
        Fields::Named(fields) => named_fields_as_struct(variant, fields, ident, enum_ident),
        Fields::Unnamed(fields) => unnamed_fields_as_struct(variant, fields, ident, enum_ident),
        _ => unreachable!(),
    }
}

pub fn named_fields_as_struct(
    variant: &Variant,
    fields: &FieldsNamed,
    ident: &Ident,
    enum_ident: &Ident,
) -> Result<TokenStream> {
    let variant_ident = &variant.ident;
    let as_enum = {
        let fieldname = fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();
        quote! {
            #enum_ident::#variant_ident {
                #(
                    #fieldname: self.#fieldname,
                    )*
            }
        }
    };
    let as_struct = syn::ItemStruct {
        attrs: variant.attrs.clone(),
        vis: syn::Visibility::Inherited,
        struct_token: Default::default(),
        ident: ident.clone(),
        generics: Default::default(),
        fields: variant.fields.clone(),
        semi_token: None,
    };
    Ok(quote! {
        #[derive(Deserialize)]
        #as_struct

        impl #ident {
            fn as_enum(self) -> #enum_ident {
                #as_enum
            }
        }
    })
}

pub fn unnamed_fields_as_struct(
    variant: &Variant,
    fields: &FieldsUnnamed,
    ident: &Ident,
    enum_ident: &Ident,
) -> Result<TokenStream> {
    let variant_ident = &variant.ident;
    let field_idents = (0..fields.unnamed.len())
        .map(|x| Ident::new(&format!("__f{}", x), Span::call_site()))
        .collect::<Vec<_>>();
    let field_types = fields.unnamed.iter().map(|f| &f.ty).collect::<Vec<_>>();
    let as_struct = quote! {
        struct #ident {
            #(#field_idents: #field_types,)*
        }
    };
    let de_impl = if fields.unnamed.len() == 1 {
        let ty = field_types[0];
        quote! {
            impl miniserde::Deserialize for #ident {
                fn begin(__out: &mut miniserde::export::Option<Self>) -> &mut dyn miniserde::de::Visitor {
                    <#ty as miniserde::Deserialize>::begin(unsafe {&mut *{__out as *mut miniserde::export::Option<Self> as *mut miniserde::export::Option<#ty>}})
                }
            }
        }
    } else {
        let index = 0usize..;
        quote! {
            struct __Visitor {
                __out: miniserde::export::Option<#ident>,
            }

            impl miniserde::Deserialize for #ident {
                fn begin(__out: &mut miniserde::export::Option<Self>) -> &mut dyn miniserde::de::Visitor {
                    unsafe {
                        &mut *{
                            __out as *mut miniserde::export::Option<Self>
                                as *mut __Visitor
                        }
                    }
                }
            }

            impl miniserde::de::Visitor for __Visitor {
                fn seq(&mut self) -> miniserde::Result<miniserde::export::Box<dyn miniserde::de::Seq + '_>> {
                    Ok(miniserde::export::Box::new(__State {
                        #(#field_idents: None,)*
                        __state: 0,
                        __out: &mut self.__out,
                    }))
                }
            }

            struct __State<'a> {
                #(#field_idents: miniserde::export::Option<#field_types>,)*
                __state: usize,
                __out: &'a mut miniserde::export::Option<#ident>,
            }

            impl<'a> miniserde::de::Seq for __State<'a> {
                fn element(&mut self) -> miniserde::Result<&mut dyn miniserde::de::Visitor> {
                    let state = self.__state;
                    self.__state += 1;
                    match state {
                        #(#index => Ok(<#field_types as miniserde::Deserialize>::begin(&mut self.#field_idents)),)*
                        _ => Err(miniserde::Error),
                    }
                }

                fn finish(&mut self) -> miniserde::Result<()> {
                    *self.__out = Some(#ident{
                        #(#field_idents: match self.#field_idents.take() {
                            Some(f) => f,
                            None => return Err(miniserde::Error),
                        },)*
                    });
                    Ok(())
                }
            }
        }
    };
    Ok(quote! {
        #as_struct

        impl #ident {
            fn as_enum(self) -> #enum_ident {
                #enum_ident::#variant_ident(#(self.#field_idents,)*)
            }
        }

        const _: () = {
            #de_impl
        };
    })
}
