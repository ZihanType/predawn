use proc_macro2::TokenStream;
use quote::quote;
use quote_use::quote_use;
use syn::{DeriveInput, Field, Ident};

use crate::{serde_attr::SerdeAttr, util};

pub(crate) fn generate(input: DeriveInput) -> syn::Result<TokenStream> {
    let DeriveInput {
        attrs,
        ident,
        generics,
        data,
        ..
    } = input;

    let named = util::extract_named_struct_fields(data, "Multipart")?;

    let mut struct_field_idents = Vec::new();
    let mut define_vars = Vec::new();
    let mut parse_fields = Vec::new();
    let mut extract_vars = Vec::new();
    let mut errors = Vec::new();

    named
        .into_iter()
        .for_each(|field| match generate_single_field(field) {
            Ok((struct_field, define_var, parse_field, extract_var)) => {
                struct_field_idents.push(struct_field);
                define_vars.push(define_var);
                parse_fields.push(parse_field);
                extract_vars.push(extract_var);
            }
            Err(e) => errors.push(e),
        });

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

    let description = util::extract_description(&attrs);
    let description = util::generate_optional_lit_str(&description)
        .unwrap_or_else(|| quote!(::core::option::Option::None));

    let expand = quote_use! {
        # use core::default::Default;
        # use std::vec::Vec;
        # use std::collections::BTreeMap;
        # use predawn::{MultiRequestMediaType, ToSchema};
        # use predawn::media_type::{MediaType, RequestMediaType, has_media_type, SingleMediaType};
        # use predawn::from_request::FromRequest;
        # use predawn::response_error::MultipartError;
        # use predawn::request::Head;
        # use predawn::body::RequestBody;
        # use predawn::extract::multipart::Multipart;
        # use predawn::api_request::ApiRequest;
        # use predawn::openapi::{self, Schema, Parameter};

        impl #impl_generics_with_lifetime FromRequest<'a> for #ident #ty_generics #where_clause {
            type Error = MultipartError;

            async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
                let mut multipart = <Multipart as FromRequest<_>>::from_request(head, body).await?;

                #(#define_vars)*

                while let Some(field) = multipart.next_field().await? {
                    #(#parse_fields)*
                }

                #(#extract_vars)*

                Ok(Self { #(#struct_field_idents),* })
            }
        }

        impl #impl_generics ApiRequest for #ident #ty_generics #where_clause {
            fn parameters(_: &mut BTreeMap<String, Schema>) -> Option<Vec<openapi::Parameter>> {
                None
            }

            fn request_body(schemas: &mut BTreeMap<String, Schema>) -> Option<openapi::RequestBody> {
                Some(openapi::RequestBody {
                    description: #description,
                    content: <Self as MultiRequestMediaType>::content(schemas),
                    required: true,
                    extensions: Default::default(),
                })
            }
        }

        impl #impl_generics MediaType for #ident #ty_generics #where_clause {
            const MEDIA_TYPE: &'static str = "multipart/form-data";
        }

        impl #impl_generics RequestMediaType for #ident #ty_generics #where_clause {
            fn check_content_type(content_type: &str) -> bool {
                has_media_type(content_type, "multipart", "form-data", "form-data", None)
            }
        }

        impl #impl_generics SingleMediaType for #ident #ty_generics #where_clause {
            fn media_type(schemas: &mut BTreeMap<String, Schema>) -> openapi::MediaType {
                openapi::MediaType {
                    schema: Some(<Self as ToSchema>::schema_ref(schemas)),
                    example: Default::default(),
                    examples: Default::default(),
                    encoding: Default::default(),
                    extensions: Default::default(),
                }
            }
        }
    };

    Ok(expand)
}

fn generate_single_field(
    field: Field,
) -> syn::Result<(Ident, TokenStream, TokenStream, TokenStream)> {
    let Field {
        attrs, ident, ty, ..
    } = field;

    let SerdeAttr { rename } = SerdeAttr::new(&attrs)?;

    let struct_field_ident = ident.expect("unreachable: named field must have an identifier");

    let multipart_field = rename.unwrap_or_else(|| struct_field_ident.to_string());

    let define_var = quote_use! {
        # use core::default::Default;
        # use predawn::extract::multipart::ParseField;

        let mut #struct_field_ident = <<#ty as ParseField>::Holder as Default>::default();
    };

    let parse_field = quote_use! {
        # use predawn::extract::multipart::ParseField;

        if field.name() == Some(#multipart_field) {
            #struct_field_ident = <#ty as ParseField>::parse_field(#struct_field_ident, field, #multipart_field).await?;
            continue;
        }
    };

    let extract_var = quote_use! {
        # use predawn::extract::multipart::ParseField;

        let #struct_field_ident = <#ty as ParseField>::extract(#struct_field_ident, #multipart_field)?;
    };

    Ok((struct_field_ident, define_var, parse_field, extract_var))
}
