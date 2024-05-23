mod controller;
mod method;
mod multi_request_media_type;
mod multi_response;
mod multi_response_media_type;
mod multipart;
mod serde_attr;
mod single_response;
mod tag;
mod to_parameters;
mod to_schema;
mod util;

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

#[doc = include_str!("docs/multi_request_media_type.md")]
#[proc_macro_derive(MultiRequestMediaType, attributes(multi_request_media_type))]
pub fn multi_request_media_type(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    multi_request_media_type::generate(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[doc = include_str!("docs/multi_response_media_type.md")]
#[proc_macro_derive(MultiResponseMediaType, attributes(multi_response_media_type))]
pub fn multi_response_media_type(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    multi_response_media_type::generate(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[doc = include_str!("docs/single_response.md")]
#[proc_macro_derive(SingleResponse, attributes(single_response, header))]
pub fn single_response(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    single_response::generate(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[doc = include_str!("docs/multi_response.md")]
#[proc_macro_derive(MultiResponse, attributes(multi_response, status))]
pub fn multi_response(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    multi_response::generate(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[doc = include_str!("docs/multipart.md")]
#[proc_macro_derive(Multipart, attributes(multipart))]
pub fn multipart(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    multipart::generate(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[doc = include_str!("docs/tag.md")]
#[proc_macro_derive(Tag, attributes(tag))]
pub fn tag(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    tag::generate(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
