use from_attr::FlagOrValue;
use proc_macro2::TokenStream;
use quote::quote;
use quote_use::quote_use;
use syn::{parse_quote, Attribute, Expr, ExprLit, Lit, Meta, MetaNameValue, Path, Type};

fn get_crate_name() -> TokenStream {
    #[cfg(not(feature = "schema"))]
    quote! { ::predawn }
    #[cfg(feature = "schema")]
    quote! { ::predawn_schema }
}

pub fn extract_description(attrs: &[Attribute]) -> String {
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

pub fn generate_string_expr(s: &str) -> Expr {
    parse_quote! {
        ::std::string::ToString::to_string(#s)
    }
}

pub fn generate_default_expr(
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

pub fn generate_add_default_to_schema(ty: &Type, default_expr: Option<Expr>) -> TokenStream {
    let crate_name = get_crate_name();

    match default_expr {
        Some(expr) => quote_use! {
            # use std::{concat, stringify, file, line, column};
            # use #crate_name::__internal::serde_json;

            schema.schema_data.default = Some(
                serde_json::to_value::<#ty>(#expr)
                    .expect(concat!("failed to serialize `", stringify!(#ty), "` type at ", file!(), ":", line!(), ":", column!()))
            );
        },
        None => TokenStream::new(),
    }
}
