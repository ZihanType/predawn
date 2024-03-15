use std::{borrow::Cow, collections::BTreeMap};

use bytes::{Bytes, BytesMut};
use http::StatusCode;

use crate::{
    body::ResponseBody,
    media_type::MultiResponseMediaType,
    openapi::{self, Components},
};

pub type Response<T = ResponseBody> = http::Response<T>;

pub trait SingleResponse {
    const STATUS_CODE: u16 = 200;

    fn response(components: &mut Components) -> openapi::Response;
}

pub trait MultiResponse {
    fn responses(components: &mut Components) -> BTreeMap<StatusCode, openapi::Response>;
}

impl<T: SingleResponse> MultiResponse for T {
    fn responses(components: &mut Components) -> BTreeMap<StatusCode, openapi::Response> {
        let mut map = BTreeMap::new();

        map.insert(
            StatusCode::from_u16(T::STATUS_CODE).unwrap_or_else(|_| {
                panic!(
                    "`<{} as SingleResponse>::STATUS_CODE` is {}, which is not a valid status code",
                    std::any::type_name::<T>(),
                    T::STATUS_CODE
                )
            }),
            T::response(components),
        );

        map
    }
}

impl SingleResponse for () {
    fn response(_: &mut Components) -> openapi::Response {
        openapi::Response::default()
    }
}

macro_rules! some_impl {
    ($ty:ty; $($desc:tt)+) => {
        impl $($desc)+
        {
            fn response(components: &mut Components) -> openapi::Response {
                openapi::Response {
                    content: <$ty as MultiResponseMediaType>::content(components),
                    ..Default::default()
                }
            }
        }
    };
}

some_impl!(String; SingleResponse for &'static str);
some_impl!(String; SingleResponse for Cow<'static, str>);
some_impl!(String; SingleResponse for String);
some_impl!(String; SingleResponse for Box<str>);

some_impl!(Vec<u8>; SingleResponse for &'static [u8]);
some_impl!(Vec<u8>; SingleResponse for Cow<'static, [u8]>);
some_impl!(Vec<u8>; SingleResponse for Vec<u8>);
some_impl!(Vec<u8>; SingleResponse for Bytes);
some_impl!(Vec<u8>; SingleResponse for BytesMut);
some_impl!(Vec<u8>; SingleResponse for Box<[u8]>);

some_impl!([u8; N]; <const N: usize> SingleResponse for [u8; N]);
some_impl!([u8; N]; <const N: usize> SingleResponse for &'static [u8; N]);
