use std::{borrow::Cow, error, fmt};

use bytes::{Bytes, BytesMut};
use indexmap::IndexMap;
use mime::{Mime, APPLICATION, CHARSET, OCTET_STREAM, PLAIN, TEXT, UTF_8};
use predawn_schema::ToSchema;

use crate::openapi::{Components, MediaType};

#[derive(Debug)]
pub struct InvalidContentType(pub String);

impl fmt::Display for InvalidContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl error::Error for InvalidContentType {}

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
        && (mime.subtype() == subtype || mime.suffix().map_or(false, |name| name == suffix));

    if let Some((key, value)) = param {
        has = has && mime.get_param(key).map_or(false, |name| name == value);
    }

    has
}

pub trait SingleMediaType {
    const MEDIA_TYPE: &'static str;

    fn media_type(components: &mut Components) -> MediaType;
}

pub trait SingleRequestMediaType: SingleMediaType {
    fn check_content_type(content_type: &str) -> bool;
}

pub trait SingleResponseMediaType: SingleMediaType {}

pub trait MultiRequestMediaType {
    fn content(components: &mut Components) -> IndexMap<String, MediaType>;
}

pub trait MultiResponseMediaType {
    fn content(components: &mut Components) -> IndexMap<String, MediaType>;
}

macro_rules! content_impl {
    ($ty:ty) => {
        fn content(components: &mut Components) -> IndexMap<String, MediaType> {
            let mut map = IndexMap::with_capacity(1);
            map.insert(<$ty>::MEDIA_TYPE.to_string(), <$ty>::media_type(components));
            map
        }
    };
}

impl<T: SingleRequestMediaType> MultiRequestMediaType for T {
    content_impl!(T);
}

impl<T: SingleResponseMediaType> MultiResponseMediaType for T {
    content_impl!(T);
}

macro_rules! impl_for_str {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl SingleMediaType for $ty {
                const MEDIA_TYPE: &'static str = "text/plain; charset=utf-8";

                fn media_type(components: &mut Components) -> MediaType {
                    MediaType {
                        schema: Some(<String as ToSchema>::schema_ref(components)),
                        ..Default::default()
                    }
                }
            }

            impl SingleRequestMediaType for $ty {
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

            impl SingleResponseMediaType for $ty {}
        )+
    };
}

impl_for_str![&'static str, Cow<'static, str>, String, Box<str>];

macro_rules! impl_for_bytes {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl SingleMediaType for $ty {
                const MEDIA_TYPE: &'static str = "application/octet-stream";

                fn media_type(components: &mut Components) -> MediaType {
                    MediaType {
                        schema: Some(<Vec<u8> as ToSchema>::schema_ref(components)),
                        ..Default::default()
                    }
                }
            }

            impl SingleRequestMediaType for $ty {
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

            impl SingleResponseMediaType for $ty {}
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
            impl<const N: usize> SingleMediaType for $ty {
                const MEDIA_TYPE: &'static str = <Vec<u8> as SingleMediaType>::MEDIA_TYPE;

                fn media_type(components: &mut Components) -> MediaType {
                    MediaType {
                        schema: Some(<[u8; N] as ToSchema>::schema_ref(components)),
                        ..Default::default()
                    }
                }
            }

            impl<const N: usize> SingleRequestMediaType for $ty {
                fn check_content_type(content_type: &str) -> bool {
                    <Vec<u8> as SingleRequestMediaType>::check_content_type(content_type)
                }
            }

            impl<const N: usize> SingleResponseMediaType for $ty {}
        )+
    };
}

impl_for_const_n_usize![[u8; N], &'static [u8; N]];
