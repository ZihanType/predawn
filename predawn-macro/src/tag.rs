use from_attr::{AttrsValue, FromAttr};
use proc_macro2::TokenStream;
use quote::quote;
use quote_use::quote_use;
use syn::DeriveInput;

#[derive(FromAttr, Default)]
#[attribute(idents = [tag])]
struct TypeAttr {
    rename: Option<String>,
}

pub(crate) fn generate(input: DeriveInput) -> syn::Result<TokenStream> {
    let DeriveInput { attrs, ident, .. } = input;

    let TypeAttr { rename } = match TypeAttr::from_attributes(&attrs) {
        Ok(Some(AttrsValue {
            value: type_attr, ..
        })) => type_attr,
        Ok(None) => Default::default(),
        Err(AttrsValue { value: e, .. }) => return Err(e),
    };

    let ident_str = rename.unwrap_or_else(|| ident.to_string());

    let description = predawn_macro_core::util::extract_description(&attrs);
    let description = if description.is_empty() {
        quote! { None }
    } else {
        let description = predawn_macro_core::util::generate_string_expr(&description);
        quote! { Some(#description) }
    };

    let expand = quote_use! {
        # use core::default::Default;
        # use std::string::ToString;
        # use predawn::Tag;
        # use predawn::openapi;

        impl Tag for #ident {
            const NAME: &'static str = #ident_str;

            fn create() -> openapi::Tag {
                openapi::Tag {
                    name: ToString::to_string(Self::NAME),
                    description: #description,
                    external_docs: Default::default(),
                    extensions: Default::default(),
                }
            }
        }
    };

    Ok(expand)
}
