use crate::attr;
use crate::TagType;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{DataEnum, DeriveInput, Ident, Error, Fields, FieldsNamed, FieldsUnnamed, Result};

pub fn derive(input: &DeriveInput, enumeration: &DataEnum) -> Result<TokenStream> {
    if input.generics.lt_token.is_some() || input.generics.where_clause.is_some() {
        return Err(Error::new(
            Span::call_site(),
            "Enums with generics are not supported",
        ));
    }
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
                    quote!{
                        #ident::#var_ident => {#implementation}
                    }
                }
                Fields::Named(fields) => {
                    let implementation = serialize_named(&fields, name, &tag_type)?;
                    let field_ident = fields
                        .named
                        .iter()
                        .map(|field| &field.ident)
                        .collect::<Vec<_>>();

                    quote!{
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
                    let implementation = serialize_unnamed(fields, &field_ident, name, &tag_type)?;
                    quote!{
                        #ident::#var_ident(#(#field_ident),*) => {
                            #implementation
                        }
                    }
                }
            })
        }).collect::<Result<Vec<_>>>()?;

    Ok(quote!{
        const _: () = {
            impl miniserde::Serialize for #ident {
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
        quote!{
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
        quote!{miniserde::ser::Fragment::Str(miniserde::export::Cow::Borrowed(#variant_name))}
    })
}

fn serialize_named(
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
    Ok(if let TagType::External = tag_type {
        quote!{
            use miniserde::Serialize;
            #[derive(Serialize)]
            struct __AsStruct<'__b> {
                #(#field_ident: &'__b #field_type),*,
            }

            struct __SuperMap<'__a> {
                data: __AsStruct<'__a>,
                state: miniserde::export::usize,
            }

            impl<'__a> miniserde::ser::Map for __SuperMap<'__a> {
                fn next(&mut self) -> miniserde::export::Option<(miniserde::export::Cow<miniserde::export::str>, &dyn miniserde::Serialize)> {
                    let __state = self.state;
                    self.state = __state + 1;
                    match __state {
                        0 => miniserde::export::Some((
                            miniserde::export::Cow::Borrowed(#variant_name),
                            &self.data,
                        )),
                        _ => miniserde::export::None,
                    }
                }
            }

            miniserde::ser::Fragment::Map(miniserde::export::Box::new(__SuperMap {
                data: __AsStruct { #(#field_ident),* },
                state: 0,
            }))
        }
    } else {
        let (start, tag_arm) = if let TagType::Internal(ref tag) = &tag_type {
            (
                0usize,
                quote!{
                    0 => miniserde::export::Some((
                        miniserde::export::Cow::Borrowed(#tag),
                        &#variant_name,
                    )),
                },
            )
        } else {
            (1, quote!())
        };
        let index = 1usize..;
        quote!{
            struct __Map<'__a> {
                #(#field_ident: &'__a #field_type),*,
                state: miniserde::export::usize,
            }

            impl<'__a> miniserde::ser::Map for __Map<'__a> {
                fn next(&mut self) -> miniserde::export::Option<(miniserde::export::Cow<miniserde::export::str>, &dyn miniserde::Serialize)> {
                    let __state = self.state;
                    self.state = __state + 1;
                    match __state {
                        #tag_arm
                        #(#index => {
                            miniserde::export::Some((
                                miniserde::export::Cow::Borrowed(#field_name),
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
        }
    })
}

fn serialize_unnamed(
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
    let index = 0usize..;
    let seq = quote!{
        struct __Seq<'__a> {
            #(#field_ident: &'__a #field_type),*,
            state: miniserde::export::usize,
        }

        impl<'__a> miniserde::ser::Seq for __Seq<'__a> {
            fn next(&mut self) -> miniserde::export::Option<&dyn miniserde::Serialize> {
                let __state = self.state;
                self.state = __state + 1;
                match __state {
                    #(#index => {
                        miniserde::export::Some(self.#field_ident)
                    })*,
                    _ => miniserde::export::None,
                }
            }
        }
    };
    Ok(if let TagType::External = tag_type {
        quote!{
            #seq

            struct __AsStruct<'__a> (#(&'__a #field_type),*);

            impl<'__a> miniserde::Serialize for __AsStruct<'__a> {
                fn begin(&self) -> miniserde::ser::Fragment {
                    let __AsStruct(#(#field_ident),*) = self;
                    miniserde::ser::Fragment::Seq(miniserde::export::Box::new(__Seq {
                        #(#field_ident),*,
                        state: 0,
                    }))
                }
            }

            struct __SuperMap<'__a> {
                data: __AsStruct<'__a>,
                state: bool,
            }

            impl<'__a> miniserde::ser::Map for __SuperMap<'__a> {
                fn next(&mut self) -> miniserde::export::Option<(miniserde::export::Cow<miniserde::export::str>, &dyn miniserde::Serialize)> {
                    if self.state {
                        return miniserde::export::None;
                    }
                    self.state = true;
                    miniserde::export::Some((
                            miniserde::export::Cow::Borrowed(#variant_name),
                            &self.data,
                    ))
                }
            }

            miniserde::ser::Fragment::Map(miniserde::export::Box::new(__SuperMap {
                data: __AsStruct ( #(#field_ident),* ),
                state: false,
            }))
        }
    } else {
        quote!{
            #seq

            miniserde::ser::Fragment::Seq(miniserde::export::Box::new(__Seq {
                #(#field_ident),*,
                state: 0,
            }))
        }
    })
}
