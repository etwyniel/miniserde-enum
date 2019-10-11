use proc_macro2::{TokenStream, Span};
use syn::{
    DeriveInput, Result, DataEnum, Error, Fields, FieldsNamed,
};
use quote::quote;
use crate::{Variant, TagType};
use crate::attr;

pub fn derive(input: &DeriveInput, enumeration: &DataEnum) -> Result<TokenStream> {
    if input.generics.lt_token.is_some() || input.generics.where_clause.is_some() {
        return Err(Error::new(
            Span::call_site(),
            "Enums with generics are not supported",
        ));
    }
    let ident = &input.ident;
    let variants: Vec<Variant> = enumeration.variants.iter().map(Into::into).collect();
    let tag_type = attr::tag_type(&input.attrs, &enumeration)?;
    let names = enumeration.variants
        .iter()
        .map(attr::name_of_variant)
        .collect::<Result<Vec<_>>>()?;
    let begin = enumeration.variants
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
                    let field_ident = fields.named.iter().map(|field| &field.ident).collect::<Vec<_>>();

                    quote!{
                        #ident::#var_ident{#(#field_ident),*} => {
                            #implementation
                        }
                    }
                },
                Fields::Unnamed(fields) => quote!{_ => unimplemented!(),},
            })
        })
        .collect::<Result<Vec<_>>>()?;

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

fn serialize_named(fields: &FieldsNamed, variant_name: &str, tag_type: &TagType) -> Result<TokenStream> {
    let field_ident = fields.named.iter().map(|field| &field.ident).collect::<Vec<_>>();
    let field_name = fields.named.iter().map(attr::name_of_field).collect::<Result<Vec<_>>>()?;
    let field_type = fields.named.iter().map(|field| &field.ty).collect::<Vec<_>>();
    Ok(if let TagType::External = tag_type {
        quote!{
            use miniserde::Serialize;
            #[derive(Serialize)]
            struct __AsStruct<'__b> {
                #(#field_ident: &'__b #field_type),*,
            }

            struct __SuperMap<'__b> {
                data: __AsStruct<'__b>,
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
            (0usize, quote!{
                0 => miniserde::export::Some((
                        miniserde::export::Cow::Borrowed(#tag),
                        &#variant_name,
                        )),
            })
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
