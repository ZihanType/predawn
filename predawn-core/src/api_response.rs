use std::{borrow::Cow, collections::BTreeMap, convert::Infallible};

use bytes::{Bytes, BytesMut};
use http::StatusCode;

use crate::{
    body::ResponseBody,
    openapi::{self, Schema},
    response::{MultiResponse, Response},
};

pub trait ApiResponse {
    fn responses(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> Option<BTreeMap<StatusCode, openapi::Response>>;
}

impl<B> ApiResponse for Response<B> {
    fn responses(
        _: &mut BTreeMap<String, Schema>,
        _: &mut Vec<String>,
    ) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        None
    }
}

macro_rules! none_response {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ApiResponse for $ty {
                fn responses(_: &mut BTreeMap<String, Schema>, _: &mut Vec<String>) -> Option<BTreeMap<StatusCode, openapi::Response>> {
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
                fn responses(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Option<BTreeMap<StatusCode, openapi::Response>> {
                    Some(<$ty as MultiResponse>::responses(schemas, schemas_in_progress))
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
                fn responses(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Option<BTreeMap<StatusCode, openapi::Response>> {
                    Some(<$ty as MultiResponse>::responses(schemas, schemas_in_progress))
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
    fn responses(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        T::responses(schemas, schemas_in_progress)
    }
}
