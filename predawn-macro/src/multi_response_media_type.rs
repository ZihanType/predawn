use from_attr::{AttrsValue, FromAttr, PathValue};
use http::StatusCode;
use proc_macro2::TokenStream;
use quote_use::quote_use;
use syn::{spanned::Spanned, DeriveInput, Ident, Type, Variant};

#[derive(FromAttr)]
#[attribute(idents = [multi_response_media_type])]
struct EnumAttr {
    error: Type,
    status_code: PathValue<u16>,
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
        status_code:
            PathValue {
                path: status_code_span,
                value: status_code_value,
            },
    } =
        match EnumAttr::from_attributes(&attrs) {
            Ok(Some(AttrsValue {
                value: enum_attr, ..
            })) => enum_attr,
            Ok(None) => return Err(syn::Error::new(
                ident.span(),
                "must have `#[multi_response_media_type(error = SomeIntoResponseError)]` attribute",
            )),
            Err(AttrsValue { value: e, .. }) => return Err(e),
        };

    if StatusCode::from_u16(status_code_value).is_err() {
        return Err(syn::Error::new(
            status_code_span,
            "it is not a valid status code",
        ));
    }

    let variants = crate::util::extract_variants(data, "MultiRequestMediaType")?;

    let variants_size = variants.len();
    let mut content_bodies = Vec::new();
    let mut into_response_arms = Vec::new();
    let mut errors = Vec::new();

    for variant in variants.into_iter() {
        match handle_single_variant(variant, &ident) {
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
        # use predawn::media_type::MultiResponseMediaType;
        # use predawn::openapi::{self, Components, MediaType};
        # use predawn::__internal::indexmap::IndexMap;
        # use predawn::response::{SingleResponse, Response, MultiResponse};
        # use predawn::into_response::IntoResponse;
        # use predawn::__internal::http::StatusCode;

        impl #impl_generics MultiResponseMediaType for #ident #ty_generics #where_clause {
            fn content(
                components: &mut Components,
            ) -> IndexMap<String, MediaType> {
                let mut map = IndexMap::with_capacity(#variants_size);
                #(#content_bodies)*
                map
            }
        }

        impl #impl_generics SingleResponse for #ident #ty_generics #where_clause {
            const STATUS_CODE: u16 = #status_code_value;

            fn response(components: &mut Components) -> openapi::Response {
                openapi::Response {
                    content: <Self as MultiResponseMediaType>::content(components),
                    ..Default::default()
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

            fn responses(
                components: &mut Components,
            ) -> Option<BTreeMap<StatusCode, openapi::Response>> {
                Some(<Self as MultiResponse>::responses(components))
            }
        }
    };

    Ok(expand)
}

fn handle_single_variant(
    variant: Variant,
    enum_ident: &Ident,
) -> syn::Result<(TokenStream, TokenStream)> {
    let variant_span = variant.span();

    let Variant {
        ident: variant_ident,
        fields,
        ..
    } = variant;

    let ty = crate::util::extract_single_unnamed_field_type_from_variant(fields, variant_span)?;

    let content_body = quote_use! {
        # use std::string::ToString;
        # use predawn::media_type::SingleMediaType;

        map.insert(
            ToString::to_string(<#ty as SingleMediaType>::MEDIA_TYPE),
            <#ty as SingleMediaType>::media_type(components),
        );
    };

    let into_response_arm = quote_use! {
        # use predawn::into_response::IntoResponse;

        #enum_ident::#variant_ident(a) => <#ty as IntoResponse>::into_response(a)?,
    };

    Ok((content_body, into_response_arm))
}
