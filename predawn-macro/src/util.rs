use http::StatusCode;
use proc_macro2::Span;
use syn::{
    punctuated::Punctuated, spanned::Spanned, Data, DataEnum, DataStruct, DataUnion, Fields,
    FieldsNamed, FieldsUnnamed, LitInt, Token, Type, Variant,
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
            format!("`{}` can only be derived for enums", derive_macro_name),
        )),
        Data::Union(DataUnion { union_token, .. }) => Err(syn::Error::new(
            union_token.span,
            format!("`{}` can only be derived for enums", derive_macro_name),
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

            if StatusCode::from_u16(status_code_value).is_err() {
                return Err(syn::Error::new(
                    status_code.span(),
                    "it is not a valid status code",
                ));
            }

            status_code_value
        }
    };

    Ok(status_code_value)
}
