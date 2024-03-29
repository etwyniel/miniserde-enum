use crate::TagType;
use syn::{Attribute, DataEnum, Error, Field, Fields, Lit, Meta, NestedMeta, Result, Variant};

pub(crate) fn tag_type(attrs: &[Attribute], enumeration: &DataEnum) -> Result<TagType> {
    let mut tag_type = None;
    let mut tag = None;
    let mut content = None;

    for attr in attrs {
        if !attr.path.is_ident("serde") {
            continue;
        }

        let list = match attr.parse_meta()? {
            Meta::List(list) => list,
            other => return Err(Error::new_spanned(other, "unsupported attribute")),
        };

        for meta in &list.nested {
            match meta {
                NestedMeta::Meta(Meta::NameValue(value)) => {
                    if value.path.is_ident("tag") {
                        if let Lit::Str(s) = &value.lit {
                            if tag.is_some() {
                                return Err(Error::new_spanned(meta, "duplicate tag attribute"));
                            }
                            tag = Some(s.value());
                            continue;
                        }
                    } else if value.path.is_ident("content") {
                        if let Lit::Str(s) = &value.lit {
                            if content.is_some() {
                                return Err(Error::new_spanned(
                                    meta,
                                    "duplicate content attribute",
                                ));
                            }
                            content = Some(s.value());
                            continue;
                        }
                    }
                }
                NestedMeta::Meta(Meta::Path(path)) => {
                    if path.is_ident("untagged") {
                        if tag_type.is_some() {
                            return Err(Error::new_spanned(meta, "duplicate tag attribute"));
                        }
                        tag_type = Some(TagType::Untagged);
                        continue;
                    }
                }
                _ => (),
            }
            return Err(Error::new_spanned(meta, "unsupported attribute"));
        }
    }
    if let Some(ty) = tag_type {
        return Ok(ty);
    }

    match (tag, content) {
        (None, None) => Ok(TagType::External),
        (Some(tag), None) => {
            for fields in enumeration.variants.iter().map(|v| &v.fields) {
                if let Fields::Unnamed(_) = fields {
                    return Err(Error::new_spanned(
                        fields,
                        "enums containing tuple variants cannot be internally tagged",
                    ));
                }
            }
            Ok(TagType::Internal(tag))
        }
        (Some(tag), Some(content)) => Ok(TagType::Adjacent { tag, content }),
        _ => Err(Error::new_spanned(
            &attrs[0],
            "Invalid enum representation.",
        )),
    }
}

/// Find the value of a #[serde(rename = "...")] attribute.
fn attr_rename(attrs: &[Attribute]) -> Result<Option<String>> {
    let mut rename = None;

    for attr in attrs {
        if !attr.path.is_ident("serde") {
            continue;
        }

        let list = match attr.parse_meta()? {
            Meta::List(list) => list,
            other => return Err(Error::new_spanned(other, "unsupported attribute")),
        };

        for meta in &list.nested {
            if let NestedMeta::Meta(Meta::NameValue(value)) = meta {
                if value.path.is_ident("rename") {
                    if let Lit::Str(s) = &value.lit {
                        if rename.is_some() {
                            return Err(Error::new_spanned(meta, "duplicate rename attribute"));
                        }
                        rename = Some(s.value());
                        continue;
                    }
                }
            }
            return Err(Error::new_spanned(meta, "unsupported attribute"));
        }
    }

    Ok(rename)
}

/// Determine the name of a field, respecting a rename attribute.
pub fn name_of_field(field: &Field) -> Result<String> {
    let rename = attr_rename(&field.attrs)?;
    Ok(rename.unwrap_or_else(|| field.ident.as_ref().unwrap().to_string()))
}

/// Determine the name of a variant, respecting a rename attribute.
pub fn name_of_variant(var: &Variant) -> Result<String> {
    let rename = attr_rename(&var.attrs)?;
    Ok(rename.unwrap_or_else(|| var.ident.to_string()))
}
