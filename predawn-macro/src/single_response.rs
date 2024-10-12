use std::collections::HashMap;

use from_attr::{AttrsValue, FromAttr};
use http::HeaderName;
use proc_macro2::TokenStream;
use quote::quote;
use quote_use::quote_use;
use syn::{
    parse_quote, spanned::Spanned, Attribute, Data, DataEnum, DataStruct, DataUnion, DeriveInput,
    Expr, ExprLit, Field, Fields, FieldsNamed, FieldsUnnamed, Generics, Ident, Lit, LitInt, Member,
    Type,
};

use crate::util;

#[derive(FromAttr, Default)]
#[attribute(idents = [single_response])]
struct StructAttr {
    status: Option<LitInt>,
}

pub(crate) fn generate(input: DeriveInput) -> syn::Result<TokenStream> {
    let DeriveInput {
        attrs,
        ident,
        generics,
        data,
        ..
    } = input;

    let StructAttr {
        status: status_code,
    } = match StructAttr::from_attributes(&attrs) {
        Ok(Some(AttrsValue {
            value: struct_attr, ..
        })) => struct_attr,
        Ok(None) => Default::default(),
        Err(AttrsValue { value: e, .. }) => return Err(e),
    };

    let status_code_value = util::extract_status_code_value(status_code)?;

    let fields = match data {
        Data::Struct(DataStruct { fields, .. }) => fields,
        Data::Enum(DataEnum { enum_token, .. }) => {
            return Err(syn::Error::new(
                enum_token.span,
                "`SingleResponse` can only be derived for structs",
            ))
        }
        Data::Union(DataUnion { union_token, .. }) => {
            return Err(syn::Error::new(
                union_token.span,
                "`SingleResponse` can only be derived for structs",
            ))
        }
    };

    let fields = match fields {
        Fields::Named(FieldsNamed { named, .. }) if !named.is_empty() => named,
        Fields::Unnamed(FieldsUnnamed { unnamed, .. }) if !unnamed.is_empty() => unnamed,
        _ => return Ok(generate_unit(&ident, status_code_value)),
    };

    let fields_len = fields.len();
    let mut fields = fields.into_iter();

    let last = fields
        .next_back()
        .expect("unreachable: fields is not empty");

    let mut header_names = HashMap::new();

    let mut insert_api_headers = Vec::new();
    let mut insert_http_headers = Vec::new();
    let mut errors = Vec::new();

    fields.enumerate().for_each(|(idx, field)| {
        match handle_single_field(field, idx, &mut header_names) {
            Ok((insert_api_header, insert_http_header)) => {
                insert_api_headers.push(insert_api_header);
                insert_http_headers.push(insert_http_header);
            }
            Err(e) => {
                errors.push(e);
            }
        }
    });

    let description = predawn_macro_core::util::extract_description(&attrs);
    let description = predawn_macro_core::util::generate_string_expr(&description);

    match handle_last_field(last, fields_len - 1, &mut header_names) {
        Ok(Last::Header {
            insert_api_header,
            insert_http_header,
        }) => {
            insert_api_headers.push(insert_api_header);
            insert_http_headers.push(insert_http_header);

            if let Some(e) = errors.into_iter().reduce(|mut a, b| {
                a.combine(b);
                a
            }) {
                return Err(e);
            }

            let expand = generate_only_headers(
                &generics,
                &ident,
                status_code_value,
                description,
                insert_api_headers,
                insert_http_headers,
            );

            Ok(expand)
        }
        Ok(Last::Body { member, ty }) => {
            let into_response_arg = parse_quote!(self.#member);

            if let Some(e) = errors.into_iter().reduce(|mut a, b| {
                a.combine(b);
                a
            }) {
                return Err(e);
            }

            let expand = if insert_api_headers.is_empty() {
                generate_only_body(
                    &generics,
                    &ident,
                    status_code_value,
                    description,
                    ty,
                    into_response_arg,
                )
            } else {
                generate_body_and_headers(
                    &generics,
                    &ident,
                    status_code_value,
                    description,
                    insert_api_headers,
                    insert_http_headers,
                    ty,
                    into_response_arg,
                )
            };

            Ok(expand)
        }
        Err(e) => {
            errors.push(e);

            let e = errors
                .into_iter()
                .reduce(|mut a, b| {
                    a.combine(b);
                    a
                })
                .expect("unreachable: errors at least one element");

            Err(e)
        }
    }
}

fn generate_unit(ident: &Ident, status_code_value: u16) -> TokenStream {
    quote_use! {
        # use core::default::Default;
        # use std::collections::BTreeMap;
        # use predawn::{SingleResponse, MultiResponse};
        # use predawn::into_response::IntoResponse;
        # use predawn::api_response::ApiResponse;
        # use predawn::response::Response;
        # use predawn::openapi::{self, Schema};
        # use predawn::http::StatusCode;

        impl SingleResponse for #ident {
            const STATUS_CODE: u16 = #status_code_value;

            fn response(_: &mut BTreeMap<String, Schema>, _: &mut Vec<String>) -> openapi::Response {
                Default::default()
            }
        }

        impl IntoResponse for #ident {
            type Error = <() as IntoResponse>::Error;

            fn into_response(self) -> Result<Response, Self::Error> {
                let mut response = <() as IntoResponse>::into_response(())?;
                *response.status_mut() = StatusCode::from_u16(#status_code_value).unwrap();
                Ok(response)
            }
        }

        impl ApiResponse for #ident {
            fn responses(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Option<BTreeMap<StatusCode, openapi::Response>> {
                Some(<Self as MultiResponse>::responses(schemas, schemas_in_progress))
            }
        }
    }
}

fn generate_only_headers(
    generics: &Generics,
    ident: &Ident,
    status_code_value: u16,
    description: Expr,
    insert_api_headers: Vec<TokenStream>,
    insert_http_headers: Vec<TokenStream>,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let headers_len = insert_api_headers.len();

    quote_use! {
        # use core::default::Default;
        # use std::collections::BTreeMap;
        # use predawn::{SingleResponse, MultiResponse};
        # use predawn::into_response::IntoResponse;
        # use predawn::api_response::ApiResponse;
        # use predawn::response::Response;
        # use predawn::openapi::{self, Schema};
        # use predawn::response_error::InvalidHeaderValue;
        # use predawn::__internal::indexmap::IndexMap;
        # use predawn::http::StatusCode;

        impl #impl_generics SingleResponse for #ident #ty_generics #where_clause {
            const STATUS_CODE: u16 = #status_code_value;

            fn response(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> openapi::Response {
                let mut headers = IndexMap::with_capacity(#headers_len);

                #(#insert_api_headers)*

                openapi::Response {
                    description: #description,
                    headers,
                    content: Default::default(),
                    links: Default::default(),
                    extensions: Default::default(),
                }
            }
        }

        impl #impl_generics IntoResponse for #ident #ty_generics #where_clause {
            type Error = InvalidHeaderValue;

            fn into_response(self) -> Result<Response, <Self as IntoResponse>::Error> {
                let mut response = <() as IntoResponse>::into_response(()).unwrap();

                *response.status_mut() = StatusCode::from_u16(#status_code_value).unwrap();

                #(
                    let _: () = #insert_http_headers?;
                )*

                Ok(response)
            }
        }

        impl #impl_generics ApiResponse for #ident #ty_generics #where_clause {
            fn responses(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Option<BTreeMap<StatusCode, openapi::Response>> {
                Some(<Self as MultiResponse>::responses(schemas, schemas_in_progress))
            }
        }
    }
}

fn generate_only_body(
    generics: &Generics,
    ident: &Ident,
    status_code_value: u16,
    description: Expr,
    body_type: Type,
    into_response_arg: Expr,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote_use! {
        # use core::default::Default;
        # use std::collections::BTreeMap;
        # use predawn::{SingleResponse, MultiResponse};
        # use predawn::into_response::IntoResponse;
        # use predawn::api_response::ApiResponse;
        # use predawn::response::Response;
        # use predawn::MultiResponseMediaType;
        # use predawn::openapi::{self, Schema};
        # use predawn::http::StatusCode;

        impl #impl_generics SingleResponse for #ident #ty_generics #where_clause {
            const STATUS_CODE: u16 = #status_code_value;

            fn response(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> openapi::Response {
                openapi::Response {
                    description: #description,
                    headers: Default::default(),
                    content: <#body_type as MultiResponseMediaType>::content(schemas, schemas_in_progress),
                    links: Default::default(),
                    extensions: Default::default(),
                }
            }
        }

        impl #impl_generics IntoResponse for #ident #ty_generics #where_clause {
            type Error = <#body_type as IntoResponse>::Error;

            fn into_response(self) -> Result<Response, <Self as IntoResponse>::Error> {
                let mut response = <#body_type as IntoResponse>::into_response(#into_response_arg)?;
                *response.status_mut() = StatusCode::from_u16(#status_code_value).unwrap();
                Ok(response)
            }
        }

        impl #impl_generics ApiResponse for #ident #ty_generics #where_clause {
            fn responses(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Option<BTreeMap<StatusCode, openapi::Response>> {
                Some(<Self as MultiResponse>::responses(schemas, schemas_in_progress))
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_body_and_headers(
    generics: &Generics,
    ident: &Ident,
    status_code_value: u16,
    description: Expr,
    insert_api_headers: Vec<TokenStream>,
    insert_http_headers: Vec<TokenStream>,
    body_type: Type,
    into_response_arg: Expr,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let headers_len = insert_api_headers.len();

    quote_use! {
        # use core::default::Default;
        # use std::collections::BTreeMap;
        # use predawn::{SingleResponse, MultiResponse};
        # use predawn::into_response::IntoResponse;
        # use predawn::api_response::ApiResponse;
        # use predawn::response::Response;
        # use predawn::MultiResponseMediaType;
        # use predawn::openapi::{self, Schema};
        # use predawn::either::Either;
        # use predawn::response_error::InvalidHeaderValue;
        # use predawn::__internal::indexmap::IndexMap;
        # use predawn::http::StatusCode;

        impl #impl_generics SingleResponse for #ident #ty_generics #where_clause {
            const STATUS_CODE: u16 = #status_code_value;

            fn response(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> openapi::Response {
                let mut headers = IndexMap::with_capacity(#headers_len);

                #(#insert_api_headers)*

                openapi::Response {
                    description: #description,
                    headers,
                    content: <#body_type as MultiResponseMediaType>::content(schemas, schemas_in_progress),
                    links: Default::default(),
                    extensions: Default::default(),
                }
            }
        }

        impl #impl_generics IntoResponse for #ident #ty_generics #where_clause {
            type Error = Either<<#body_type as IntoResponse>::Error, InvalidHeaderValue>;

            fn into_response(self) -> Result<Response, <Self as IntoResponse>::Error> {
                let mut response = <#body_type as IntoResponse>::into_response(#into_response_arg).map_err(Either::Left)?;

                *response.status_mut() = StatusCode::from_u16(#status_code_value).unwrap();

                #(
                    let _: () = #insert_http_headers.map_err(Either::Right)?;
                )*

                Ok(response)
            }
        }

        impl #impl_generics ApiResponse for #ident #ty_generics #where_clause {
            fn responses(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Option<BTreeMap<StatusCode, openapi::Response>> {
                Some(<Self as MultiResponse>::responses(schemas, schemas_in_progress))
            }
        }
    }
}

fn handle_single_field(
    field: Field,
    idx: usize,
    header_names: &mut HashMap<String, String>,
) -> syn::Result<(TokenStream, TokenStream)> {
    let span = field.span();

    let Field {
        attrs, ident, ty, ..
    } = field;

    let Some(header_name) = extract_header_name(&attrs, header_names)? else {
        let e = syn::Error::new(span, "missing `#[header = \"xxx\"]` attribute");
        return Err(e);
    };

    let member = match ident {
        Some(ident) => Member::from(ident),
        None => Member::from(idx),
    };

    let description = predawn_macro_core::util::extract_description(&attrs);
    let description = if description.is_empty() {
        quote! { None }
    } else {
        let description = predawn_macro_core::util::generate_string_expr(&description);
        quote! { Some(#description) }
    };

    Ok(generate_headers(&ty, &header_name, &member, description))
}

enum Last {
    Header {
        insert_api_header: TokenStream,
        insert_http_header: TokenStream,
    },
    Body {
        member: Member,
        ty: Type,
    },
}

fn handle_last_field(
    field: Field,
    idx: usize,
    header_names: &mut HashMap<String, String>,
) -> syn::Result<Last> {
    let Field {
        attrs, ident, ty, ..
    } = field;

    let member = match ident {
        Some(ident) => Member::from(ident),
        None => Member::from(idx),
    };

    let Some(header_name) = extract_header_name(&attrs, header_names)? else {
        return Ok(Last::Body { member, ty });
    };

    let description = predawn_macro_core::util::extract_description(&attrs);
    let description = if description.is_empty() {
        quote! { None }
    } else {
        let description = predawn_macro_core::util::generate_string_expr(&description);
        quote! { Some(#description) }
    };

    let (insert_api_header, insert_http_header) =
        generate_headers(&ty, &header_name, &member, description);

    Ok(Last::Header {
        insert_api_header,
        insert_http_header,
    })
}

fn generate_headers<'a>(
    ty: &'a Type,
    header_name: &'a str,
    member: &'a Member,
    description: TokenStream,
) -> (TokenStream, TokenStream) {
    let insert_api_header = quote_use! {
        # use core::default::Default;
        # use std::string::ToString;
        # use predawn::openapi::{Header, ParameterSchemaOrContent, ReferenceOr};
        # use predawn::ToSchema;

        let header = Header {
            description: #description,
            style: Default::default(),
            required: <#ty as ToSchema>::REQUIRED,
            deprecated: Default::default(),
            format: ParameterSchemaOrContent::Schema(<#ty as ToSchema>::schema_ref(schemas, schemas_in_progress)),
            example: Default::default(),
            examples: Default::default(),
            extensions: Default::default(),
        };

        headers.insert(ToString::to_string(#header_name), ReferenceOr::Item(header));
    };

    let insert_http_header = quote_use! {
        # use predawn::response::{MaybeHeaderValue, ToHeaderValue};
        # use predawn::ToSchema;
        # use predawn::outcome::Outcome;
        # use predawn::response_error::InvalidHeaderValue;
        # use predawn::http::HeaderName;

        match <#ty as ToHeaderValue>::to_header_value(&self.#member) {
            MaybeHeaderValue::Value(val) => {
                response.headers_mut().insert(HeaderName::from_static(#header_name), val);
                Ok(())
            }
            MaybeHeaderValue::Error => {
                Err(InvalidHeaderValue::error(#header_name, &self.#member))
            },
            MaybeHeaderValue::None => {
                if <#ty as ToSchema>::REQUIRED {
                    Err(InvalidHeaderValue::none(#header_name, &self.#member))
                } else {
                    Ok(())
                }
            }
        }
    };

    (insert_api_header, insert_http_header)
}

fn extract_header_name<'a>(
    attrs: &'a [Attribute],
    header_names: &'a mut HashMap<String, String>,
) -> syn::Result<Option<String>> {
    let mut errors = Vec::new();
    let mut found = None;

    for attr in attrs {
        if !attr.path().is_ident("header") {
            continue;
        }

        if found.is_some() {
            let e = syn::Error::new(attr.span(), "only one `header` attribute is allowed");
            errors.push(e);
            continue;
        }

        let value = match attr.meta.require_name_value() {
            Ok(name_value) => &name_value.value,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };

        let Expr::Lit(ExprLit {
            lit: Lit::Str(lit_str),
            ..
        }) = value
        else {
            let e = syn::Error::new(value.span(), "only string literal is allowed");
            errors.push(e);
            continue;
        };

        let raw_header_name = lit_str.value();
        let lit_str_span = lit_str.span();

        let header_name = match HeaderName::from_bytes(raw_header_name.as_bytes()) {
            Ok(header_name) => header_name,
            Err(e) => {
                errors.push(syn::Error::new(lit_str_span, e));
                continue;
            }
        };

        let header_name = header_name.as_str().to_string();

        match header_names.get(&header_name) {
            None => {
                header_names.insert(header_name.clone(), raw_header_name);
                found = Some(header_name);
            }
            Some(existing_raw_header_name) => {
                let e = syn::Error::new(
                    lit_str_span,
                    format!("duplicate with header name `{}`", existing_raw_header_name),
                );
                errors.push(e);
            }
        }
    }

    if let Some(e) = errors.into_iter().reduce(|mut a, b| {
        a.combine(b);
        a
    }) {
        return Err(e);
    }

    Ok(found)
}
