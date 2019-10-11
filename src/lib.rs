extern crate proc_macro;

mod ser;
pub(crate) mod attr;

use syn::{
    parse_macro_input, DeriveInput, Error, Data, Ident, Result,
};
use std::convert::From;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

#[derive(Debug)]
pub(crate) struct Variant {
    ident: Ident,
    name: Option<String>,
}

impl From<&syn::Variant> for Variant {
    fn from(v: &syn::Variant) -> Self {
        let ident = v.ident.clone();
        Variant {
            ident,
            name: None,
        }
    }
}

#[derive(Debug)]
enum TagType {
    External,
    Internal(String),
    Untagged,
}

#[proc_macro_derive(Serialize_enum, attributes(serde))]
pub fn derive_serialize(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);
    let en = match &input.data {
        Data::Enum(en) => en,
        _ => return Error::new_spanned(input, "Serialize_enum can only be applied to enums")
            .to_compile_error().into()
    };
    ser::derive(&input, &en).unwrap_or_else(|err| err.to_compile_error()).into()
}
