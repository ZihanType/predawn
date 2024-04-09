use std::{borrow::Cow, collections::BTreeMap, convert::Infallible};

use bytes::{Bytes, BytesMut};
use http::StatusCode;

use crate::{
    body::ResponseBody,
    openapi::{self, Components},
    response::{MultiResponse, Response},
};

pub trait ApiResponse {
    fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>>;
}

impl<B> ApiResponse for Response<B> {
    fn responses(_: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        None
    }
}

macro_rules! none_response {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ApiResponse for $ty {
                fn responses(_: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
                    None
                }
            }
        )+
    };
}

none_response![http::response::Parts, ResponseBody, StatusCode, Infallible];

macro_rules! some_response {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ApiResponse for $ty {
                fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
                    Some(<$ty as MultiResponse>::responses(components))
                }
            }
        )+
    };
}

some_response![
    (),
    // string
    &'static str,
    Cow<'static, str>,
    String,
    Box<str>,
    // bytes
    &'static [u8],
    Cow<'static, [u8]>,
    Vec<u8>,
    Box<[u8]>,
    Bytes,
    BytesMut,
];

macro_rules! const_n_response {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl<const N: usize> ApiResponse for $ty {
                fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
                    Some(<$ty as MultiResponse>::responses(components))
                }
            }
        )+
    };
}

const_n_response![[u8; N], &'static [u8; N]];

impl<T, E> ApiResponse for Result<T, E>
where
    T: ApiResponse,
{
    fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        T::responses(components)
    }
}
