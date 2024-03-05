mod controller;
mod method;
mod serde_attr;
mod to_parameters;
mod to_schema;

use from_attr::FromAttr;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemImpl};

#[proc_macro_attribute]
pub fn controller(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = match controller::ImplAttr::from_tokens(attr.into()) {
        Ok(attr) => attr,
        Err(e) => return e.to_compile_error().into(),
    };
    let item_impl = parse_macro_input!(item as ItemImpl);

    controller::generate(attr, item_impl)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(ToSchema, attributes(schema))]
pub fn to_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    to_schema::generate(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(ToParameters, attributes(parameters))]
pub fn to_parameters(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    to_parameters::generate(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
