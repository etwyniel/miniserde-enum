extern crate proc_macro;

pub(crate) mod attr;
mod bound;
mod de;
mod ser;

use std::convert::From;
use syn::{parse_macro_input, Data, DeriveInput, Error};

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
        _ => {
            return Error::new_spanned(input, "Serialize_enum can only be applied to enums")
                .to_compile_error()
                .into()
        }
    };
    ser::derive(&input, &en)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_derive(Deserialize_enum, attributes(serde))]
pub fn derive_deserialize(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);
    let en = match &input.data {
        Data::Enum(en) => en,
        _ => {
            return Error::new_spanned(input, "Serialize_enum can only be applied to enums")
                .to_compile_error()
                .into()
        }
    };
    de::derive(&input, &en)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
