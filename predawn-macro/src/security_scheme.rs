use from_attr::{AttrsValue, FromAttr, FromIdent};
use http::HeaderName;
use proc_macro2::TokenStream;
use quote::quote;
use quote_use::quote_use;
use syn::{parse_quote, spanned::Spanned, Attribute, DeriveInput, Ident, LitStr};

use crate::util;

#[derive(FromAttr)]
#[attribute(idents = [api_key])]
struct ApiKeyAttr {
    rename: Option<String>,
    #[attribute(rename = "in")]
    location: Location,
    name: LitStr,
}

#[derive(FromIdent)]
enum Location {
    Query,
    Header,
    Cookie,
}

impl Location {
    fn as_ident(&self) -> Ident {
        match self {
            Location::Query => parse_quote!(Query),
            Location::Header => parse_quote!(Header),
            Location::Cookie => parse_quote!(Cookie),
        }
    }
}

#[derive(FromAttr)]
#[attribute(idents = [http])]
struct HttpAttr {
    rename: Option<String>,
    scheme: AuthScheme,
    bearer_format: Option<String>,
}

#[derive(FromIdent)]
enum AuthScheme {
    Basic,
    Bearer,
    Digest,
    Dpop,
    Gnap,
    Hoba,
    Mutual,
}

impl AuthScheme {
    fn as_str(&self) -> &'static str {
        match self {
            AuthScheme::Basic => "Basic",
            AuthScheme::Bearer => "Bearer",
            AuthScheme::Digest => "Digest",
            AuthScheme::Dpop => "DPoP",
            AuthScheme::Gnap => "GNAP",
            AuthScheme::Hoba => "HOBA",
            AuthScheme::Mutual => "Mutual",
        }
    }
}

pub(crate) fn generate(input: DeriveInput) -> syn::Result<TokenStream> {
    let DeriveInput {
        attrs: ty_attrs,
        ident,
        ..
    } = input;

    let mut path_and_expand = None;
    let mut errors = Vec::new();

    macro_rules! expand_scheme {
        ($ty:ty, $ident:ident) => {
            match <$ty as FromAttr>::from_attributes(&ty_attrs) {
                Ok(Some(AttrsValue {
                    attrs,
                    value: api_key_attr,
                })) => {
                    match &path_and_expand {
                        Some((path, _)) => {
                            let msg = format!("already defined as {:?}", path);

                            attrs.iter().for_each(|attr| {
                                errors.push(syn::Error::new(attr.span(), &msg));
                            });
                        }
                        None => match $ident(&ty_attrs, &ident, api_key_attr) {
                            Ok(expand) => {
                                path_and_expand = Some((attrs.first().unwrap().path(), expand));
                            }
                            Err(e) => errors.push(e),
                        },
                    };
                }
                Ok(None) => {}
                Err(AttrsValue { value: e, .. }) => errors.push(e),
            }
        };
    }

    expand_scheme!(ApiKeyAttr, generate_api_key);
    expand_scheme!(HttpAttr, generate_http);

    if path_and_expand.is_none() {
        errors.push(syn::Error::new(
            ident.span(),
            "missing `#[api_key(..)]` or `#[http(..)]` attribute",
        ));
    }

    if let Some(e) = errors.into_iter().reduce(|mut a, b| {
        a.combine(b);
        a
    }) {
        return Err(e);
    }

    let (_, expand) = path_and_expand.unwrap();

    Ok(expand)
}

fn generate_api_key(
    attrs: &[Attribute],
    ident: &Ident,
    api_key: ApiKeyAttr,
) -> syn::Result<TokenStream> {
    let ApiKeyAttr {
        rename,
        location,
        name,
    } = api_key;

    let name = match location {
        Location::Header => {
            let name_str = name.value();

            match HeaderName::from_bytes(name_str.as_bytes()) {
                Ok(header_name) => header_name.as_str().to_string(),
                Err(e) => return Err(syn::Error::new(name.span(), e)),
            }
        }
        _ => name.value(),
    };

    let ident_str = rename.unwrap_or_else(|| ident.to_string());

    let location = location.as_ident();

    let description = util::extract_description(attrs);
    let description = util::generate_optional_lit_str(&description)
        .unwrap_or_else(|| quote!(::core::option::Option::None));

    let expand = quote_use! {
        # use core::default::Default;
        # use std::string::ToString;
        # use predawn::SecurityScheme;
        # use predawn::openapi::{self, APIKeyLocation};

        impl SecurityScheme for #ident {
            const NAME: &'static str = #ident_str;

            fn create() -> openapi::SecurityScheme {
                openapi::SecurityScheme::APIKey {
                    location: APIKeyLocation::#location,
                    name: ToString::to_string(#name),
                    description: #description,
                    extensions: Default::default(),
                }
            }
        }
    };

    Ok(expand)
}

fn generate_http(attrs: &[Attribute], ident: &Ident, http: HttpAttr) -> syn::Result<TokenStream> {
    let HttpAttr {
        rename,
        scheme,
        bearer_format,
    } = http;

    let ident_str = rename.unwrap_or_else(|| ident.to_string());

    let scheme = scheme.as_str();

    let bearer_format = match bearer_format {
        Some(f) if !f.is_empty() => {
            quote!(::core::option::Option::Some(::std::string::ToString::to_string(#f)))
        }
        _ => quote!(::core::option::Option::None),
    };

    let description = util::extract_description(attrs);
    let description = util::generate_optional_lit_str(&description)
        .unwrap_or_else(|| quote!(::core::option::Option::None));

    let expand = quote_use! {
        # use core::default::Default;
        # use std::string::ToString;
        # use predawn::SecurityScheme;
        # use predawn::openapi::{self, APIKeyLocation};

        impl SecurityScheme for #ident {
            const NAME: &'static str = #ident_str;

            fn create() -> openapi::SecurityScheme {
                openapi::SecurityScheme::HTTP {
                    scheme: ToString::to_string(#scheme),
                    bearer_format: #bearer_format,
                    description: #description,
                    extensions: Default::default(),
                }
            }
        }
    };

    Ok(expand)
}
