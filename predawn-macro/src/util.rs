use http::StatusCode;
use proc_macro2::Span;
use syn::{
    punctuated::Punctuated, spanned::Spanned, Attribute, Data, DataEnum, DataStruct, DataUnion,
    Field, Fields, FieldsNamed, FieldsUnnamed, LitInt, Token, Type, Variant,
};

pub(crate) fn extract_variants(
    data: Data,
    derive_macro_name: &'static str,
) -> syn::Result<Punctuated<Variant, Token![,]>> {
    match data {
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

            Ok(variants)
        }
        Data::Struct(DataStruct { struct_token, .. }) => Err(syn::Error::new(
            struct_token.span,
            format!("`{derive_macro_name}` can only be derived for enums"),
        )),
        Data::Union(DataUnion { union_token, .. }) => Err(syn::Error::new(
            union_token.span,
            format!("`{derive_macro_name}` can only be derived for enums"),
        )),
    }
}

pub(crate) fn extract_named_struct_fields(
    data: Data,
    derive_macro_name: &'static str,
) -> syn::Result<Punctuated<Field, Token![,]>> {
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
                    Ok(named)
                }
            }
            Fields::Unnamed(FieldsUnnamed { paren_token, .. }) => Err(syn::Error::new(
                paren_token.span.join(),
                format!("`{derive_macro_name}` can only be derived for structs with named fields"),
            )),
            Fields::Unit => Err(syn::Error::new(
                struct_token.span,
                format!("`{derive_macro_name}` can only be derived for structs with named fields"),
            )),
        },
        Data::Enum(DataEnum { enum_token, .. }) => Err(syn::Error::new(
            enum_token.span,
            format!("`{derive_macro_name}` can only be derived for structs"),
        )),
        Data::Union(DataUnion { union_token, .. }) => Err(syn::Error::new(
            union_token.span,
            format!("`{derive_macro_name}` can only be derived for structs"),
        )),
    }
}

pub(crate) fn extract_single_unnamed_field_type_from_variant(
    fields: Fields,
    variant_span: Span,
) -> syn::Result<Type> {
    match fields {
        Fields::Unnamed(FieldsUnnamed {
            paren_token,
            unnamed,
        }) => {
            let mut unnamed = unnamed.into_iter();

            let field = match unnamed.next() {
                Some(field) => field,
                None => {
                    return Err(syn::Error::new(
                        paren_token.span.join(),
                        "must have one field",
                    ))
                }
            };

            if let Some(e) = unnamed
                .map(|field| syn::Error::new(field.span(), "only have one field"))
                .reduce(|mut a, b| {
                    a.combine(b);
                    a
                })
            {
                return Err(e);
            }

            Ok(field.ty)
        }
        Fields::Named(FieldsNamed { brace_token, .. }) => Err(syn::Error::new(
            brace_token.span.join(),
            "only support unnamed fields",
        )),
        Fields::Unit => Err(syn::Error::new(variant_span, "only support unnamed fields")),
    }
}

pub(crate) fn extract_status_code_value(status_code: Option<LitInt>) -> syn::Result<u16> {
    let status_code_value = match status_code {
        None => 200,
        Some(status_code) => {
            let status_code_value = status_code.base10_parse()?;

            if let Err(e) = StatusCode::from_u16(status_code_value) {
                return Err(syn::Error::new(status_code.span(), e));
            }

            status_code_value
        }
    };

    Ok(status_code_value)
}

pub(crate) fn extract_summary_and_description(attrs: &[Attribute]) -> (String, String) {
    let docs = predawn_macro_core::util::extract_description(attrs);

    if docs.is_empty() {
        return (String::new(), String::new());
    }

    match docs.split_once("\n\n") {
        Some((summary, description)) => (summary.to_string(), description.to_string()),
        None => (docs, String::new()),
    }
}
