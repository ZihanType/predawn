use from_attr::FlagOrValue;
use proc_macro2::TokenStream;
use quote::quote;
use quote_use::quote_use;
use syn::{Attribute, Expr, ExprLit, Lit, Meta, MetaNameValue, Path, Type, parse_quote};

#[doc(hidden)]
pub fn get_crate_name() -> TokenStream {
    #[cfg(feature = "__used_in_predawn")]
    quote! { ::predawn }

    #[cfg(all(
        feature = "__used_in_predawn_schema",
        not(feature = "__used_in_predawn")
    ))]
    quote! { ::predawn_schema }

    #[cfg(not(any(feature = "__used_in_predawn", feature = "__used_in_predawn_schema")))]
    compile_error!(
        "either `__used_in_predawn` or `__used_in_predawn_schema` feature must be enabled"
    );
}

pub fn extract_description(attrs: &[Attribute]) -> String {
    let mut docs = String::new();

    attrs.iter().for_each(|attr| {
        if !attr.path().is_ident("doc") {
            return;
        }

        let Meta::NameValue(MetaNameValue {
            value: Expr::Lit(ExprLit {
                lit: Lit::Str(doc), ..
            }),
            ..
        }) = &attr.meta
        else {
            return;
        };

        let doc = doc.value();

        if !docs.is_empty() {
            docs.push('\n');
        }

        docs.push_str(doc.trim());
    });

    docs
}

pub fn remove_description(attrs: &mut Vec<Attribute>) {
    attrs.retain(|attr| !attr.path().is_ident("doc"));
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

pub fn generate_json_value(ty: &Type, expr: &Expr) -> TokenStream {
    let crate_name = get_crate_name();

    quote_use! {
        # use std::{concat, stringify, file, line, column};
        # use #crate_name::__internal::serde_json;

        serde_json::to_value::<#ty>(#expr)
            .expect(concat!(
                "failed to serialize expression `", stringify!(#expr), "` of type `", stringify!(#ty),
                "`, at ", file!(), ":", line!(), ":", column!()
            ))
    }
}
