use std::{borrow::Cow, collections::BTreeMap};

use bytes::{Bytes, BytesMut};
use indexmap::IndexMap;
use mime::{APPLICATION, CHARSET, Mime, OCTET_STREAM, PLAIN, TEXT, UTF_8};
use predawn_schema::ToSchema;

use crate::openapi::{self, Schema};

#[doc(hidden)]
pub fn assert_response_media_type<T: ResponseMediaType>() {}

pub fn has_media_type<'a>(
    content_type: &'a str,
    ty: &'a str,
    subtype: &'a str,
    suffix: &'a str,
    param: Option<(&'a str, &'a str)>,
) -> bool {
    let Ok(mime) = content_type.parse::<Mime>() else {
        return false;
    };

    let mut has = mime.type_() == ty
        && (mime.subtype() == subtype || mime.suffix().is_some_and(|name| name == suffix));

    if let Some((key, value)) = param {
        has = has && mime.get_param(key).is_some_and(|name| name == value);
    }

    has
}

pub trait MediaType {
    const MEDIA_TYPE: &'static str;
}

pub trait RequestMediaType {
    fn check_content_type(content_type: &str) -> bool;
}

pub trait ResponseMediaType {}

pub trait SingleMediaType {
    fn media_type(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> openapi::MediaType;
}

pub trait MultiRequestMediaType {
    fn content(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> IndexMap<String, openapi::MediaType>;
}

pub trait MultiResponseMediaType {
    fn content(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> IndexMap<String, openapi::MediaType>;
}

macro_rules! default_impl {
    ($bound:ident, $trait:ident) => {
        impl<T> $trait for T
        where
            T: MediaType + SingleMediaType + $bound,
        {
            fn content(
                schemas: &mut BTreeMap<String, Schema>,
                schemas_in_progress: &mut Vec<String>,
            ) -> IndexMap<String, openapi::MediaType> {
                let mut map = IndexMap::with_capacity(1);
                map.insert(
                    T::MEDIA_TYPE.to_string(),
                    T::media_type(schemas, schemas_in_progress),
                );
                map
            }
        }
    };
}

default_impl!(RequestMediaType, MultiRequestMediaType);
default_impl!(ResponseMediaType, MultiResponseMediaType);

macro_rules! impl_for_str {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl MediaType for $ty {
                const MEDIA_TYPE: &'static str = "text/plain; charset=utf-8";
            }

            impl RequestMediaType for $ty {
                fn check_content_type(content_type: &str) -> bool {
                    has_media_type(
                        content_type,
                        TEXT.as_str(),
                        PLAIN.as_str(),
                        PLAIN.as_str(),
                        Some((CHARSET.as_str(), UTF_8.as_str())),
                    )
                }
            }

            impl ResponseMediaType for $ty {}

            impl SingleMediaType for $ty {
                fn media_type(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> openapi::MediaType {
                    openapi::MediaType {
                        schema: Some(<String as ToSchema>::schema_ref(schemas, schemas_in_progress)),
                        ..Default::default()
                    }
                }
            }
        )+
    };
}

impl_for_str![&'static str, Cow<'static, str>, String, Box<str>];

macro_rules! impl_for_bytes {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl MediaType for $ty {
                const MEDIA_TYPE: &'static str = "application/octet-stream";
            }

            impl RequestMediaType for $ty {
                fn check_content_type(content_type: &str) -> bool {
                    has_media_type(
                        content_type,
                        APPLICATION.as_str(),
                        OCTET_STREAM.as_str(),
                        OCTET_STREAM.as_str(),
                        None,
                    )
                }
            }

            impl ResponseMediaType for $ty {}

            impl SingleMediaType for $ty {
                fn media_type(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> openapi::MediaType {
                    openapi::MediaType {
                        schema: Some(<Vec<u8> as ToSchema>::schema_ref(schemas, schemas_in_progress)),
                        ..Default::default()
                    }
                }
            }
        )+
    };
}

impl_for_bytes![
    &'static [u8],
    Cow<'static, [u8]>,
    Vec<u8>,
    Bytes,
    BytesMut,
    Box<[u8]>,
];

macro_rules! impl_for_const_n_usize {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl<const N: usize> MediaType for $ty {
                const MEDIA_TYPE: &'static str = <Vec<u8> as MediaType>::MEDIA_TYPE;
            }

            impl<const N: usize> RequestMediaType for $ty {
                fn check_content_type(content_type: &str) -> bool {
                    <Vec<u8> as RequestMediaType>::check_content_type(content_type)
                }
            }

            impl<const N: usize> ResponseMediaType for $ty {}

            impl<const N: usize> SingleMediaType for $ty {
                fn media_type(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> openapi::MediaType {
                    openapi::MediaType {
                        schema: Some(<[u8; N] as ToSchema>::schema_ref(schemas, schemas_in_progress)),
                        ..Default::default()
                    }
                }
            }
        )+
    };
}

impl_for_const_n_usize![[u8; N], &'static [u8; N]];
