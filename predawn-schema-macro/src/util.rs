use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, DataStruct, DataUnion, Fields, FieldsNamed, FieldsUnnamed, Variant};

use crate::types::{SchemaFields, SchemaProperties, SchemaVariant, UnitVariant};

pub(crate) fn get_crate_name() -> TokenStream {
    #[cfg(not(feature = "schema"))]
    quote! { ::predawn }
    #[cfg(feature = "schema")]
    quote! { ::predawn_schema }
}

pub(crate) fn extract_schema_properties(data: Data) -> syn::Result<SchemaProperties> {
    match data {
        Data::Struct(DataStruct {
            struct_token,
            fields,
            ..
        }) => match fields {
            Fields::Named(FieldsNamed { brace_token, named }) => {
                if named.is_empty() {
                    Err(syn::Error::new(
                        brace_token.span.join(),
                        "must have at least one field",
                    ))
                } else {
                    Ok(SchemaProperties::NamedStruct(named))
                }
            }
            Fields::Unnamed(FieldsUnnamed { paren_token, .. }) => Err(syn::Error::new(
                paren_token.span.join(),
                "`ToSchema` can not be derived for structs with unnamed fields",
            )),
            Fields::Unit => Err(syn::Error::new(
                struct_token.span,
                "`ToSchema` can not be derived for unit structs",
            )),
        },
        Data::Enum(DataEnum {
            brace_token,
            variants,
            ..
        }) => {
            if variants.is_empty() {
                return Err(syn::Error::new(
                    brace_token.span.join(),
                    "must have at least one variant",
                ));
            }

            let only_unit = variants.iter().all(|variant| {
                let description = predawn_macro_core::util::extract_description(&variant.attrs);
                if !description.is_empty() {
                    return false;
                }

                matches!(variant.fields, Fields::Unit)
            });

            if only_unit {
                let variants = variants
                    .into_iter()
                    .map(|variant| {
                        let Variant { attrs, ident, .. } = variant;
                        UnitVariant { attrs, ident }
                    })
                    .collect::<Vec<_>>();

                Ok(SchemaProperties::OnlyUnitEnum(variants))
            } else {
                let mut errors = Vec::with_capacity(variants.len());

                let variants = variants
                    .into_iter()
                    .filter_map(|variant| {
                        let Variant {
                            attrs,
                            ident,
                            fields,
                            ..
                        } = variant;

                        match fields {
                            Fields::Named(FieldsNamed { brace_token, named }) => {
                                if named.is_empty() {
                                    errors.push(syn::Error::new(
                                        brace_token.span.join(),
                                        "must have at least one field",
                                    ));
                                    None
                                } else {
                                    Some(SchemaVariant {
                                        attrs,
                                        ident,
                                        fields: SchemaFields::Named(named),
                                    })
                                }
                            }
                            Fields::Unnamed(FieldsUnnamed {
                                paren_token,
                                mut unnamed,
                            }) => {
                                if unnamed.len() != 1 {
                                    errors.push(syn::Error::new(
                                        paren_token.span.join(),
                                        "must have only one field",
                                    ));
                                    None
                                } else {
                                    Some(SchemaVariant {
                                        attrs,
                                        ident,
                                        fields: SchemaFields::Unnamed(
                                            unnamed.pop().unwrap().into_value(),
                                        ),
                                    })
                                }
                            }
                            Fields::Unit => Some(SchemaVariant {
                                attrs,
                                ident,
                                fields: SchemaFields::Unit,
                            }),
                        }
                    })
                    .collect::<Vec<_>>();

                if let Some(e) = errors.into_iter().reduce(|mut a, b| {
                    a.combine(b);
                    a
                }) {
                    return Err(e);
                }

                Ok(SchemaProperties::NormalEnum(variants))
            }
        }
        Data::Union(DataUnion { union_token, .. }) => Err(syn::Error::new(
            union_token.span,
            "`ToSchema` can not be derived for unions",
        )),
    }
}
