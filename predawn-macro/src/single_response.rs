use std::collections::HashMap;

use from_attr::{AttrsValue, FromAttr};
use http::HeaderName;
use proc_macro2::TokenStream;
use quote_use::quote_use;
use syn::{
    parse_quote, spanned::Spanned, Attribute, Data, DataEnum, DataStruct, DataUnion, DeriveInput,
    Expr, ExprLit, Field, Fields, FieldsNamed, FieldsUnnamed, Ident, Lit, LitInt, Member, Type,
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

    let maybe_named = match fields {
        Fields::Named(FieldsNamed { named, .. }) if !named.is_empty() => named,
        Fields::Unnamed(FieldsUnnamed { unnamed, .. }) if !unnamed.is_empty() => unnamed,
        _ => return Ok(generate_unit(&ident, status_code_value)),
    };

    let fields_len = maybe_named.len();
    let mut fields = maybe_named.into_iter();

    let last = fields
        .next_back()
        .expect("unreachable: fields is not empty");

    let mut header_names = HashMap::new();

    let mut response_bodies = Vec::new();
    let mut into_response_bodies = Vec::new();
    let mut errors = Vec::new();

    fields.enumerate().for_each(|(idx, field)| {
        match handle_single_field(field, idx, &mut header_names) {
            Ok((response_body, into_response_body)) => {
                response_bodies.push(response_body);
                into_response_bodies.push(into_response_body);
            }
            Err(e) => {
                errors.push(e);
            }
        }
    });

    let content_field_value: Expr;
    let response_ty: Type;
    let into_response_arg: Expr;

    match handle_last_field(last, fields_len - 1, &mut header_names) {
        Ok(Last::Header {
            response_body,
            into_response_body,
        }) => {
            response_bodies.push(response_body);
            into_response_bodies.push(into_response_body);

            content_field_value = parse_quote!(::core::default::Default::default());
            response_ty = parse_quote!(());
            into_response_arg = parse_quote!(());
        }
        Ok(Last::Body { member, ty }) => {
            content_field_value =
                parse_quote!(<#ty as ::predawn::MultiResponseMediaType>::content(components));
            response_ty = ty;
            into_response_arg = parse_quote!(self.#member);
        }
        Err(e) => {
            errors.push(e);

            // avoid throwing `used binding `into_response_arg` is possibly-uninitialized` error messages,
            // which need to be returned early here
            let e = errors
                .into_iter()
                .reduce(|mut a, b| {
                    a.combine(b);
                    a
                })
                .expect("unreachable: errors at least one element");

            return Err(e);
        }
    }

    if let Some(e) = errors.into_iter().reduce(|mut a, b| {
        a.combine(b);
        a
    }) {
        return Err(e);
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let headers_len = response_bodies.len();

    let expand = quote_use! {
        # use core::default::Default;
        # use std::collections::BTreeMap;
        # use predawn::{SingleResponse, MultiResponse};
        # use predawn::into_response::IntoResponse;
        # use predawn::api_response::ApiResponse;
        # use predawn::response::Response;
        # use predawn::openapi::{self, Components};
        # use predawn::__internal::indexmap::IndexMap;
        # use predawn::__internal::http::StatusCode;

        impl #impl_generics SingleResponse for #ident #ty_generics #where_clause {
            const STATUS_CODE: u16 = #status_code_value;

            fn response(components: &mut Components) -> openapi::Response {
                let mut headers = IndexMap::with_capacity(#headers_len);

                #(#response_bodies)*

                openapi::Response {
                    description: Default::default(),
                    headers,
                    content: #content_field_value,
                    links: Default::default(),
                    extensions: Default::default(),
                }
            }
        }

        impl #impl_generics IntoResponse for #ident #ty_generics #where_clause {
            type Error = <#response_ty as IntoResponse>::Error;

            fn into_response(self) -> Result<Response, Self::Error> {
                let mut response = <#response_ty as IntoResponse>::into_response(#into_response_arg)?;

                *response.status_mut() = StatusCode::from_u16(#status_code_value).unwrap();

                #(#into_response_bodies)*

                Ok(response)
            }
        }

        impl #impl_generics ApiResponse for #ident #ty_generics #where_clause {
            fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
                Some(<Self as MultiResponse>::responses(components))
            }
        }
    };

    Ok(expand)
}

fn generate_unit(struct_ident: &Ident, status_code_value: u16) -> TokenStream {
    quote_use! {
        # use core::default::Default;
        # use std::collections::BTreeMap;
        # use predawn::{SingleResponse, MultiResponse};
        # use predawn::into_response::IntoResponse;
        # use predawn::api_response::ApiResponse;
        # use predawn::response::Response;
        # use predawn::openapi::{self, Components};
        # use predawn::__internal::http::StatusCode;

        impl SingleResponse for #struct_ident {
            const STATUS_CODE: u16 = #status_code_value;

            fn response(components: &mut Components) -> openapi::Response {
                Default::default()
            }
        }

        impl IntoResponse for #struct_ident {
            type Error = <() as IntoResponse>::Error;

            fn into_response(self) -> Result<Response, Self::Error> {
                let mut response = <() as IntoResponse>::into_response(())?;

                *response.status_mut() = StatusCode::from_u16(#status_code_value).unwrap();

                Ok(response)
            }
        }

        impl ApiResponse for #struct_ident {
            fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
                Some(<Self as MultiResponse>::responses(components))
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

    let Some(header) = extract_header_name(&attrs, header_names)? else {
        let e = syn::Error::new(span, "missing `#[header = \"xxx\"]` attribute");
        return Err(e);
    };

    let member = match ident {
        Some(ident) => Member::from(ident),
        None => Member::from(idx),
    };

    Ok(generate_bodies(&ty, &header, &member))
}

enum Last {
    Header {
        response_body: TokenStream,
        into_response_body: TokenStream,
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

    let Some(header) = extract_header_name(&attrs, header_names)? else {
        return Ok(Last::Body { member, ty });
    };

    let (response_body, into_response_body) = generate_bodies(&ty, &header, &member);

    Ok(Last::Header {
        response_body,
        into_response_body,
    })
}

fn generate_bodies<'a>(
    ty: &'a Type,
    header_name: &'a str,
    member: &'a Member,
) -> (TokenStream, TokenStream) {
    let response_body = quote_use! {
        # use core::default::Default;
        # use std::string::ToString;
        # use predawn::openapi::{Header, ParameterSchemaOrContent, ReferenceOr};
        # use predawn::ToSchema;

        let header = Header {
            description: Default::default(),
            style: Default::default(),
            required: <#ty as ToSchema>::REQUIRED,
            deprecated: Default::default(),
            format: ParameterSchemaOrContent::Schema(<#ty as ToSchema>::schema_ref(components)),
            example: Default::default(),
            examples: Default::default(),
            extensions: Default::default(),
        };

        headers.insert(ToString::to_string(#header_name), ReferenceOr::Item(header));
    };

    let into_response_body = quote_use! {
        # use predawn::response::{panic_on_err, panic_on_none, ToHeaderValue};
        # use predawn::ToSchema;
        # use predawn::__internal::http::HeaderName;

        match <#ty as ToHeaderValue>::to_header_value(&self.#member) {
            Some(Ok(val)) => {
                response.headers_mut().insert(HeaderName::from_static(#header_name), val);
            }
            Some(Err(_)) => panic_on_err(&self.#member),
            None => {
                if <#ty as ToSchema>::REQUIRED {
                    panic_on_none::<#ty>()
                }
            }
        }
    };

    (response_body, into_response_body)
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

        if found.is_some() {
            let e = syn::Error::new(attr.span(), "only one `header` attribute is allowed");
            errors.push(e);
            continue;
        }

        let raw_header_name = lit_str.value();
        let lit_str_span = lit_str.span();

        let Ok(header_name) = HeaderName::from_bytes(raw_header_name.as_bytes()) else {
            let e = syn::Error::new(lit_str_span, "it is not a valid header name");
            errors.push(e);
            continue;
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
