use from_attr::{AttrsValue, FromAttr};
use proc_macro2::TokenStream;
use quote_use::quote_use;
use syn::{spanned::Spanned, DeriveInput, Ident, LitInt, Type, Variant};

use crate::util;

#[derive(FromAttr)]
#[attribute(idents = [multi_response_media_type])]
struct EnumAttr {
    error: Type,
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

    let EnumAttr {
        error: into_response_error,
        status: status_code,
    } =
        match EnumAttr::from_attributes(&attrs) {
            Ok(Some(AttrsValue {
                value: enum_attr, ..
            })) => enum_attr,
            Ok(None) => return Err(syn::Error::new(
                ident.span(),
                "missing `#[multi_response_media_type(error = SomeIntoResponseError)]` attribute",
            )),
            Err(AttrsValue { value: e, .. }) => return Err(e),
        };

    let status_code_value = util::extract_status_code_value(status_code)?;

    let variants = util::extract_variants(data, "MultiRequestMediaType")?;

    let variants_size = variants.len();
    let mut content_bodies = Vec::new();
    let mut into_response_arms = Vec::new();
    let mut errors = Vec::new();

    for variant in variants.into_iter() {
        match handle_single_variant(variant, &ident, &into_response_error) {
            Ok((content_body, into_response_arm)) => {
                content_bodies.push(content_body);
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
        # use std::string::String;
        # use std::collections::BTreeMap;
        # use predawn::MultiResponseMediaType;
        # use predawn::openapi::{self, ReferenceOr, Schema};
        # use predawn::__internal::indexmap::IndexMap;
        # use predawn::response::Response;
        # use predawn::{SingleResponse, MultiResponse};
        # use predawn::into_response::IntoResponse;
        # use predawn::api_response::ApiResponse;
        # use predawn::__internal::http::StatusCode;
        # use predawn::__internal::indexmap::IndexMap;

        impl #impl_generics MultiResponseMediaType for #ident #ty_generics #where_clause {
            fn content(schemas: &mut IndexMap<String, ReferenceOr<Schema>>) -> IndexMap<String, openapi::MediaType> {
                let mut map = IndexMap::with_capacity(#variants_size);
                #(#content_bodies)*
                map
            }
        }

        impl #impl_generics SingleResponse for #ident #ty_generics #where_clause {
            const STATUS_CODE: u16 = #status_code_value;

            fn response(schemas: &mut IndexMap<String, ReferenceOr<Schema>>) -> openapi::Response {
                openapi::Response {
                    description: Default::default(),
                    headers: Default::default(),
                    content: <Self as MultiResponseMediaType>::content(schemas),
                    links: Default::default(),
                    extensions: Default::default(),
                }
            }
        }

        impl #impl_generics IntoResponse for #ident #ty_generics #where_clause {
            type Error = #into_response_error;

            fn into_response(self) -> Result<Response, Self::Error> {
                let mut response = match self {
                    #(#into_response_arms)*
                };

                *response.status_mut() = StatusCode::from_u16(#status_code_value).unwrap();

                Ok(response)
            }
        }

        impl #impl_generics ApiResponse for #ident #ty_generics #where_clause {
            fn responses(schemas: &mut IndexMap<String, ReferenceOr<Schema>>) -> Option<BTreeMap<StatusCode, openapi::Response>> {
                Some(<Self as MultiResponse>::responses(schemas))
            }
        }
    };

    Ok(expand)
}

fn handle_single_variant<'a>(
    variant: Variant,
    enum_ident: &'a Ident,
    into_response_error: &'a Type,
) -> syn::Result<(TokenStream, TokenStream)> {
    let variant_span = variant.span();

    let Variant {
        ident: variant_ident,
        fields,
        ..
    } = variant;

    let ty = util::extract_single_unnamed_field_type_from_variant(fields, variant_span)?;

    let content_body = quote_use! {
        # use std::string::ToString;
        # use predawn::media_type::{MediaType, SingleMediaType};

        map.insert(
            ToString::to_string(<#ty as MediaType>::MEDIA_TYPE),
            <#ty as SingleMediaType>::media_type(schemas),
        );
    };

    let into_response_arm = quote_use! {
        # use core::convert::From;
        # use predawn::media_type::assert_response_media_type;
        # use predawn::into_response::IntoResponse;

        #enum_ident::#variant_ident(a) => {
            assert_response_media_type::<#ty>();

            match <#ty as IntoResponse>::into_response(a) {
                Ok(response) => response,
                Err(e) => return Err(<#into_response_error as From<_>>::from(e)),
            }
        },
    };

    Ok((content_body, into_response_arm))
}
