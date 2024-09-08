use std::collections::HashSet;

use from_attr::{AttrsValue, FromAttr};
use http::StatusCode;
use proc_macro2::TokenStream;
use quote_use::quote_use;
use syn::{spanned::Spanned, Attribute, DeriveInput, Expr, ExprLit, Ident, Lit, Type, Variant};

use crate::util;

#[derive(FromAttr)]
#[attribute(idents = [multi_response])]
struct EnumAttr {
    error: Type,
}

pub(crate) fn generate(input: DeriveInput) -> syn::Result<TokenStream> {
    let DeriveInput {
        attrs,
        ident,
        generics,
        data,
        ..
    } = input;

    let EnumAttr {
        error: into_response_error,
    } = match EnumAttr::from_attributes(&attrs) {
        Ok(Some(AttrsValue {
            value: enum_attr, ..
        })) => enum_attr,
        Ok(None) => {
            return Err(syn::Error::new(
                ident.span(),
                "missing `#[multi_response(error = SomeIntoResponseError)]` attribute",
            ))
        }
        Err(AttrsValue { value: e, .. }) => return Err(e),
    };

    let variants = util::extract_variants(data, "MultiResponse")?;

    let mut status_codes = HashSet::new();
    let mut responses_bodies = Vec::new();
    let mut into_response_arms = Vec::new();
    let mut errors = Vec::new();

    for variant in variants.into_iter() {
        match handle_single_variant(variant, &ident, &into_response_error, &mut status_codes) {
            Ok((responses_body, into_response_arm)) => {
                responses_bodies.push(responses_body);
                into_response_arms.push(into_response_arm);
            }
            Err(e) => errors.push(e),
        }
    }

    if let Some(e) = errors.into_iter().reduce(|mut a, b| {
        a.combine(b);
        a
    }) {
        return Err(e);
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expand = quote_use! {
        # use std::collections::BTreeMap;
        # use predawn::MultiResponse;
        # use predawn::openapi::{self, Schema};
        # use predawn::response::Response;
        # use predawn::into_response::IntoResponse;
        # use predawn::api_response::ApiResponse;
        # use predawn::__internal::http::StatusCode;

        impl #impl_generics MultiResponse for #ident #ty_generics #where_clause {
            fn responses(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> BTreeMap<StatusCode, openapi::Response> {
                let mut map = BTreeMap::new();

                #(#responses_bodies)*

                map
            }
        }

        impl #impl_generics IntoResponse for #ident #ty_generics #where_clause {
            type Error = #into_response_error;

            fn into_response(self) -> Result<Response, <Self as IntoResponse>::Error> {
                let (mut response, status) = match self {
                    #(#into_response_arms)*
                };

                *response.status_mut() = StatusCode::from_u16(status).unwrap();

                Ok(response)
            }
        }

        impl #impl_generics ApiResponse for #ident #ty_generics #where_clause {
            fn responses(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Option<BTreeMap<StatusCode, openapi::Response>> {
                Some(<Self as MultiResponse>::responses(schemas, schemas_in_progress))
            }
        }
    };

    Ok(expand)
}

fn handle_single_variant<'a>(
    variant: Variant,
    enum_ident: &'a Ident,
    into_response_error: &'a Type,
    status_codes: &'a mut HashSet<u16>,
) -> syn::Result<(TokenStream, TokenStream)> {
    let variant_span = variant.span();

    let Variant {
        attrs,
        ident: variant_ident,
        fields,
        ..
    } = variant;

    let Some(status_code) = extract_status_code(&attrs, status_codes)? else {
        let e = syn::Error::new(variant_span, "missing `#[status = xxx]` attribute");
        return Err(e);
    };

    let ty = util::extract_single_unnamed_field_type_from_variant(fields, variant_span)?;

    let responses_body = quote_use! {
        # use predawn::SingleResponse;
        # use predawn::__internal::http::StatusCode;

        map.insert(
            StatusCode::from_u16(#status_code).unwrap(),
            <#ty as SingleResponse>::response(schemas, schemas_in_progress),
        );
    };

    let into_response_arm = quote_use! {
        # use core::convert::From;
        # use predawn::into_response::IntoResponse;

        #enum_ident::#variant_ident(a) => match <#ty as IntoResponse>::into_response(a) {
            Ok(response) => (response, #status_code),
            Err(e) => return Err(<#into_response_error as From<_>>::from(e)),
        },
    };

    Ok((responses_body, into_response_arm))
}

fn extract_status_code<'a>(
    attrs: &'a [Attribute],
    status_codes: &'a mut HashSet<u16>,
) -> syn::Result<Option<u16>> {
    let mut errors = Vec::new();
    let mut found = None;

    for attr in attrs {
        if !attr.path().is_ident("status") {
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
            lit: Lit::Int(lit_int),
            ..
        }) = value
        else {
            let e = syn::Error::new(value.span(), "only int literal is allowed");
            errors.push(e);
            continue;
        };

        if found.is_some() {
            let e = syn::Error::new(attr.span(), "only one `status` attribute is allowed");
            errors.push(e);
            continue;
        }

        let status_code = match lit_int.base10_parse::<u16>() {
            Ok(a) => a,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };

        let lit_int_span = lit_int.span();

        if StatusCode::from_u16(status_code).is_err() {
            let e = syn::Error::new(lit_int_span, "it is not a valid status code");
            errors.push(e);
            continue;
        }

        if !status_codes.contains(&status_code) {
            status_codes.insert(status_code);
            found = Some(status_code);
        } else {
            let e = syn::Error::new(lit_int_span, "duplicate status code");
            errors.push(e);
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
