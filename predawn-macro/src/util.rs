use from_attr::FlagOrValue;
use http::StatusCode;
use proc_macro2::{Span, TokenStream};
use quote_use::quote_use;
use syn::{
    parse_quote, punctuated::Punctuated, spanned::Spanned, Attribute, Data, DataEnum, DataStruct,
    DataUnion, Expr, ExprLit, Field, Fields, FieldsNamed, FieldsUnnamed, Lit, LitInt, Meta,
    MetaNameValue, Path, Token, Type, Variant,
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

pub(crate) fn extract_description(attrs: &[Attribute]) -> String {
    let mut docs = String::new();

    attrs.iter().for_each(|attr| {
        let meta = if attr.path().is_ident("doc") {
            &attr.meta
        } else {
            return;
        };

        let doc = if let Meta::NameValue(MetaNameValue {
            value: Expr::Lit(ExprLit {
                lit: Lit::Str(doc), ..
            }),
            ..
        }) = meta
        {
            doc.value()
        } else {
            return;
        };

        if !docs.is_empty() {
            docs.push('\n');
        }

        docs.push_str(doc.trim());
    });

    docs
}

pub(crate) fn extract_summary_and_description(attrs: &[Attribute]) -> (String, String) {
    let docs = extract_description(attrs);

    if docs.is_empty() {
        return (String::new(), String::new());
    }

    match docs.split_once("\n\n") {
        Some((summary, description)) => (summary.to_string(), description.to_string()),
        None => (docs, String::new()),
    }
}

pub(crate) fn generate_string_expr(s: &str) -> Expr {
    parse_quote! {
        ::std::string::ToString::to_string(#s)
    }
}

pub(crate) fn generate_default_expr(
    ty: &Type,
    serde_default: FlagOrValue<String>,
    schema_default: FlagOrValue<Expr>,
) -> syn::Result<Option<Expr>> {
    let default_expr: Expr = match (serde_default, schema_default) {
        (FlagOrValue::None, FlagOrValue::None) => return Ok(None),

        (FlagOrValue::None, FlagOrValue::Flag { .. })
        | (FlagOrValue::Flag { .. }, FlagOrValue::Flag { .. })
        | (FlagOrValue::Flag { .. }, FlagOrValue::None) => {
            parse_quote! {
                <#ty as ::core::default::Default>::default()
            }
        }

        (FlagOrValue::Value { value, .. }, FlagOrValue::None)
        | (FlagOrValue::Value { value, .. }, FlagOrValue::Flag { .. }) => {
            let path = syn::parse_str::<Path>(&value)?;

            parse_quote! {
                #path()
            }
        }

        (FlagOrValue::None, FlagOrValue::Value { value: expr, .. })
        | (FlagOrValue::Flag { .. }, FlagOrValue::Value { value: expr, .. })
        | (FlagOrValue::Value { .. }, FlagOrValue::Value { value: expr, .. }) => expr,
    };

    Ok(Some(default_expr))
}

pub(crate) fn generate_add_default_to_schema(ty: &Type, default_expr: Option<Expr>) -> TokenStream {
    match default_expr {
        Some(expr) => quote_use! {
            # use std::{concat, stringify, file, line, column};
            # use predawn::__internal::serde_json;

            schema.schema_data.default = Some(
                serde_json::to_value::<#ty>(#expr)
                    .expect(concat!("failed to serialize `", stringify!(#ty), "` type at ", file!(), ":", line!(), ":", column!()))
            );
        },
        None => TokenStream::new(),
    }
}
