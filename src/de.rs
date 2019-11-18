use crate::attr;
use crate::TagType;
use crate::bound;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{DataEnum, DeriveInput, Ident, Error, Fields, FieldsNamed, FieldsUnnamed, Result, parse_quote, Variant};

pub fn derive(input: &DeriveInput, enumeration: &DataEnum) -> Result<TokenStream> {
    let tag_type = attr::tag_type(&input.attrs, &enumeration)?;
    match &tag_type {
        TagType::External => deserialize_external(input, enumeration),
        _ => return Err(Error::new(Span::call_site(), "Only externally tagged enums are supported")),
    }
}

pub fn deserialize_external(input: &DeriveInput, enumeration: &DataEnum) -> Result<TokenStream> {
    let ident = &input.ident;
    let names = enumeration
        .variants
        .iter()
        .map(attr::name_of_variant)
        .collect::<Result<Vec<_>>>()?;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let wrapper_generics = bound::with_lifetime_bound(&input.generics, "'__a");
    let (wrapper_impl_generics, wrapper_ty_generics, _) = wrapper_generics.split_for_impl();
    let bound = parse_quote!(miniserde::Deserialize);
    let bounded_where_clause = bound::where_clause_with_bound(&input.generics, bound);

    let (struct_variants, other_variants): (Vec<_>, Vec<_>) = enumeration
        .variants
        .iter()
        .partition(|v| if let Fields::Named(_) = &v.fields {true} else {false});
    let (tuple_variants, unit_variants): (Vec<_>, Vec<_>) = other_variants
                                          .into_iter()
                                          .partition(|v| if let Fields::Unnamed(_) = &v.fields {true} else {false});

    let struct_names = struct_variants
        .iter()
        .map(|variant| Ident::new(&format!("__{}_{}_Struct", input.ident, variant.ident), Span::call_site()))
        .collect::<Vec<_>>();
    let variant_names = struct_variants
        .iter()
        .map(|variant| &variant.ident)
        .collect::<Vec<_>>();
    let structs = struct_variants
        .iter()
        .zip(struct_names.iter())
        .map(|(variant, ident)| variant_as_struct(variant, ident, &input.ident))
        .collect::<Result<Vec<_>>>()?;
    let unit_variant_idents = unit_variants
        .iter()
        .filter(|v| if let Fields::Unit = &v.fields {true} else {false})
        .map(|v| &v.ident)
        .collect::<Vec<_>>();
    let unit_variant_names = unit_variants
        .iter()
        .cloned()
        .filter(|v| if let Fields::Unit = &v.fields {true} else {false})
        .map(attr::name_of_variant)
        .collect::<Result<Vec<_>>>()?;

    Ok(quote!{
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
                        #(#variant_names: None,)*
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
                #(#variant_names: miniserde::export::Option<#struct_names>,)*
                __out: &'__a mut miniserde::export::Option<#ident #ty_generics>,
            }

            #(#structs)*

            impl #wrapper_impl_generics miniserde::de::Map for __State #wrapper_ty_generics #bounded_where_clause {
                fn key(&mut self, k: &miniserde::export::str) -> miniserde::Result<&mut dyn miniserde::de::Visitor> {
                    match k {
                        #(
                            #names => miniserde::export::Ok(#struct_names::begin(&mut self.#variant_names)),
                        )*
                            _ => miniserde::export::Ok(miniserde::de::Visitor::ignore()),
                    }
                }

                fn finish(&mut self) -> miniserde::Result<()> {
                    #(
                        if let Some(val) = self.#variant_names.take() {
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

pub fn variant_as_struct(variant: &Variant, ident: &Ident, enum_ident: &Ident) -> Result<TokenStream> {
    let variant_ident = &variant.ident;
    let as_enum = match &variant.fields {
        Fields::Named(fields) => {
            let fieldname = fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();
            let fieldty = fields.named.iter().map(|f| &f.ty);
            quote!{
                #enum_ident::#variant_ident {
                    #(
                        #fieldname: self.#fieldname,
                    )*
                }
            }
        }
        _ => quote!(unimplemented!()),
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
    Ok(quote!{
        #[derive(Deserialize)]
        #as_struct

        impl #ident {
            fn as_enum(self) -> #enum_ident {
                #as_enum
            }
        }
    })
}
