use from_attr::{AttrsValue, FromAttr};
use proc_macro2::TokenStream;
use quote::quote;
use quote_use::quote_use;
use syn::{spanned::Spanned, DeriveInput, Ident, Type, Variant};

use crate::util;

#[derive(FromAttr)]
#[attribute(idents = [multi_request_media_type])]
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
        error: from_request_error,
    } = match EnumAttr::from_attributes(&attrs) {
        Ok(Some(AttrsValue {
            value: enum_attr, ..
        })) => enum_attr,
        Ok(None) => {
            return Err(syn::Error::new(
                ident.span(),
                "missing `#[multi_request_media_type(error = SomeFromRequestError)]` attribute",
            ))
        }
        Err(AttrsValue { value: e, .. }) => return Err(e),
    };

    let variants = util::extract_variants(data, "MultiRequestMediaType")?;

    let variants_len = variants.len();
    let mut media_type_exprs = Vec::new();
    let mut content_bodies = Vec::new();
    let mut from_request_bodies = Vec::new();
    let mut errors = Vec::new();

    for variant in variants.into_iter() {
        match handle_single_variant(variant, &ident, &from_request_error) {
            Ok((media_type_expr, content_body, from_request_body)) => {
                media_type_exprs.push(media_type_expr);
                content_bodies.push(content_body);
                from_request_bodies.push(from_request_body);
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

    let impl_generics_with_lifetime = {
        let mut s = quote!(#impl_generics).to_string();
        match s.find('<') {
            Some(pos) => {
                s.insert_str(pos + 1, "'a,");
                syn::parse_str(&s).unwrap()
            }
            _ => quote!(<'a>),
        }
    };

    let description = predawn_macro_core::util::extract_description(&attrs);
    let description = if description.is_empty() {
        quote! { None }
    } else {
        let description = predawn_macro_core::util::generate_string_expr(&description);
        quote! { Some(#description) }
    };

    let expand = quote_use! {
        # use core::convert::From;
        # use std::vec::Vec;
        # use std::collections::BTreeMap;
        # use std::string::String;
        # use predawn::MultiRequestMediaType;
        # use predawn::response_error::InvalidContentType;
        # use predawn::openapi::{self, Schema, Parameter};
        # use predawn::__internal::indexmap::IndexMap;
        # use predawn::from_request::FromRequest;
        # use predawn::api_request::ApiRequest;
        # use predawn::request::Head;
        # use predawn::body::RequestBody;
        # use predawn::location::Location;

        impl #impl_generics MultiRequestMediaType for #ident #ty_generics #where_clause {
            fn content(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> IndexMap<String, openapi::MediaType> {
                let mut map = IndexMap::with_capacity(#variants_len);
                #(#content_bodies)*
                map
            }
        }

        impl #impl_generics_with_lifetime FromRequest<'a> for #ident #ty_generics #where_clause {
            type Error = #from_request_error;

            async fn from_request(head: &'a mut Head, body: RequestBody) -> Result<Self, Self::Error> {
                let content_type = head.content_type().unwrap_or_default();

                #(#from_request_bodies)*

                Err(<#from_request_error as From<_>>::from(InvalidContentType {
                    location: Location::caller(),
                    actual: content_type.into(),
                    expected: [#(#media_type_exprs,)*],
                }))
            }
        }

        impl #impl_generics ApiRequest for #ident #ty_generics #where_clause {
            fn parameters(_: &mut BTreeMap<String, Schema>, _: &mut Vec<String>) -> Option<Vec<Parameter>> {
                None
            }

            fn request_body(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Option<openapi::RequestBody> {
                Some(openapi::RequestBody {
                    description: #description,
                    content: <Self as MultiRequestMediaType>::content(schemas, schemas_in_progress),
                    required: true,
                    extensions: Default::default(),
                })
            }
        }
    };

    Ok(expand)
}

fn handle_single_variant<'a>(
    variant: Variant,
    enum_ident: &'a Ident,
    from_request_error: &'a Type,
) -> syn::Result<(TokenStream, TokenStream, TokenStream)> {
    let variant_span = variant.span();

    let Variant {
        ident: variant_ident,
        fields,
        ..
    } = variant;

    let ty = util::extract_single_unnamed_field_type_from_variant(fields, variant_span)?;

    let media_type_expr = quote_use! {
        # use predawn::media_type::MediaType;

        <#ty as MediaType>::MEDIA_TYPE
    };

    let content_body = quote_use! {
        # use std::string::ToString;
        # use predawn::media_type::SingleMediaType;

        map.insert(
            ToString::to_string(#media_type_expr),
            <#ty as SingleMediaType>::media_type(schemas, schemas_in_progress),
        );
    };

    let from_request_body = quote_use! {
        # use core::convert::From;
        # use predawn::media_type::RequestMediaType;
        # use predawn::from_request::FromRequest;

        if <#ty as RequestMediaType>::check_content_type(content_type) {
            return match <#ty as FromRequest<_>>::from_request(head, body).await {
                Ok(o) => Ok(#enum_ident::#variant_ident(o)),
                Err(e) => Err(<#from_request_error as From<_>>::from(e)),
            };
        }
    };

    Ok((media_type_expr, content_body, from_request_body))
}
