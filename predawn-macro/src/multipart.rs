use proc_macro2::TokenStream;
use quote::quote;
use quote_use::quote_use;
use syn::{DeriveInput, Field, Ident};

use crate::serde_attr::SerdeAttr;

pub(crate) fn generate(input: DeriveInput) -> syn::Result<TokenStream> {
    let DeriveInput {
        ident,
        generics,
        data,
        ..
    } = input;

    let named = crate::util::extract_named_struct_fields(data, "Multipart")?;

    let mut struct_field_idents = Vec::new();
    let mut define_vars = Vec::new();
    let mut parse_fields = Vec::new();
    let mut unwrap_vars = Vec::new();
    let mut errors = Vec::new();

    named
        .into_iter()
        .for_each(|field| match generate_single_field(field) {
            Ok((struct_field, define_var, parse_field, unwrap_var)) => {
                struct_field_idents.push(struct_field);
                define_vars.push(define_var);
                parse_fields.push(parse_field);
                unwrap_vars.push(unwrap_var);
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

    let expand = quote_use! {
        # use core::default::Default;
        # use std::vec::Vec;
        # use predawn::{MultiRequestMediaType, ToSchema};
        # use predawn::media_type::{MediaType, RequestMediaType, has_media_type, SingleMediaType};
        # use predawn::from_request::FromRequest;
        # use predawn::response_error::MultipartError;
        # use predawn::request::Head;
        # use predawn::body::RequestBody;
        # use predawn::extract::multipart::Multipart;
        # use predawn::api_request::ApiRequest;
        # use predawn::openapi::{self, Components, Parameter};

        impl #impl_generics_with_lifetime FromRequest<'a> for #ident #ty_generics #where_clause {
            type Error = MultipartError;

            async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
                let mut multipart = <Multipart as FromRequest<_>>::from_request(head, body).await?;

                #(#define_vars)*

                while let Some(field) = multipart.next_field().await? {
                    #(#parse_fields)*
                }

                #(#unwrap_vars)*

                Ok(Self { #(#struct_field_idents),* })
            }
        }

        impl #impl_generics ApiRequest for #ident #ty_generics #where_clause {
            fn parameters(_: &mut Components) -> Option<Vec<openapi::Parameter>> {
                None
            }

            fn request_body(components: &mut Components) -> Option<openapi::RequestBody> {
                Some(openapi::RequestBody {
                    description: Default::default(),
                    content: <Self as MultiRequestMediaType>::content(components),
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
            fn media_type(components: &mut Components) -> openapi::MediaType {
                openapi::MediaType {
                    schema: Some(<Self as ToSchema>::schema_ref(components)),
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
        let mut #struct_field_ident = None;
    };

    let parse_field = quote_use! {
        # use predawn::extract::multipart::ParseField;

        if field.name() == Some(#multipart_field) {
            let v = match #struct_field_ident {
                Some(v) => {
                    <#ty as ParseField>::parse_repeated_field(v, field, #multipart_field).await?
                }
                None => <#ty as ParseField>::parse_field(field, #multipart_field).await?,
            };

            #struct_field_ident = Some(v);
            continue;
        }
    };

    let unwrap_var = quote_use! {
        # use predawn::extract::multipart::ParseField;
        # use predawn::ToSchema;
        # use predawn::response_error::MultipartError;

        let #struct_field_ident = match #struct_field_ident {
            Some(v) => v,
            None => {
                let e = MultipartError::MissingField { name: #multipart_field };

                if <#ty as ToSchema>::REQUIRED {
                    return Err(e);
                }

                <#ty as ParseField>::default().ok_or(e)?
            }
        };
    };

    Ok((struct_field_ident, define_var, parse_field, unwrap_var))
}