use std::{borrow::Cow, collections::BTreeMap};

use bytes::{Bytes, BytesMut};
use http::StatusCode;
use openapiv3::Components;

use crate::{body::ResponseBody, media_type::MultiResponseMediaType};

pub type Response<T = ResponseBody> = http::Response<T>;

pub trait SingleResponse {
    const STATUS_CODE: StatusCode = StatusCode::OK;

    fn response(components: &mut Components) -> openapiv3::Response;
}

pub trait MultiResponse {
    fn responses(components: &mut Components) -> BTreeMap<StatusCode, openapiv3::Response>;
}

impl<T: SingleResponse> MultiResponse for T {
    fn responses(components: &mut Components) -> BTreeMap<StatusCode, openapiv3::Response> {
        let mut map = BTreeMap::new();
        map.insert(T::STATUS_CODE, T::response(components));
        map
    }
}

macro_rules! simple_impl {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl SingleResponse for $ty {
                fn response(_: &mut Components) -> openapiv3::Response {
                    openapiv3::Response::default()
                }
            }
        )+
    };
}

simple_impl![(), ResponseBody];

macro_rules! some_impl {
    ($ty:ty; $($desc:tt)+) => {
        impl $($desc)+
        {
            fn response(components: &mut Components) -> openapiv3::Response {
                openapiv3::Response {
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
