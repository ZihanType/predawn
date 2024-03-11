use std::{borrow::Cow, collections::BTreeMap, convert::Infallible};

use bytes::{Bytes, BytesMut};
use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};

use crate::{
    body::ResponseBody,
    either::Either,
    media_type::SingleMediaType,
    openapi::{self, Components},
    response::{MultiResponse, Response},
    response_error::ResponseError,
};

pub trait IntoResponse {
    type Error: ResponseError;

    fn into_response(self) -> Result<Response, Self::Error>;

    fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>>;
}

impl IntoResponse for Response {
    type Error = Infallible;

    fn into_response(self) -> Result<Response, Self::Error> {
        Ok(self)
    }

    fn responses(_: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        None
    }
}

impl IntoResponse for http::response::Parts {
    type Error = Infallible;

    fn into_response(self) -> Result<Response, Self::Error> {
        Ok(Response::from_parts(self, ResponseBody::empty()))
    }

    fn responses(_: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        None
    }
}

impl IntoResponse for ResponseBody {
    type Error = Infallible;

    fn into_response(self) -> Result<Response, Self::Error> {
        Ok(Response::new(self))
    }

    fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        Some(<ResponseBody as MultiResponse>::responses(components))
    }
}

impl IntoResponse for () {
    type Error = Infallible;

    fn into_response(self) -> Result<Response, Self::Error> {
        ResponseBody::empty().into_response()
    }

    fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        Some(<() as MultiResponse>::responses(components))
    }
}

impl IntoResponse for StatusCode {
    type Error = Infallible;

    fn into_response(self) -> Result<Response, Self::Error> {
        let mut response = ().into_response()?;
        *response.status_mut() = self;
        Ok(response)
    }

    fn responses(_: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        None
    }
}

impl IntoResponse for Infallible {
    type Error = Infallible;

    fn into_response(self) -> Result<Response, Self::Error> {
        match self {}
    }

    fn responses(_: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        None
    }
}

macro_rules! some_impl {
    ($ty:ty; $($desc:tt)+) => {
        impl $($desc)+
        {
            type Error = Infallible;

            fn into_response(self) -> Result<Response, Self::Error> {
                let mut response = Response::new(self.into());

                response
                    .headers_mut()
                    .insert(CONTENT_TYPE, HeaderValue::from_static(<$ty as SingleMediaType>::MEDIA_TYPE));

                Ok(response)
            }

            fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
                Some(<$ty as MultiResponse>::responses(components))
            }
        }
    };
}

some_impl!(String; IntoResponse for &'static str);
some_impl!(String; IntoResponse for Cow<'static, str>);
some_impl!(String; IntoResponse for String);
some_impl!(String; IntoResponse for Box<str>);

some_impl!(Vec<u8>; IntoResponse for &'static [u8]);
some_impl!(Vec<u8>; IntoResponse for Cow<'static, [u8]>);
some_impl!(Vec<u8>; IntoResponse for Vec<u8>);
some_impl!(Vec<u8>; IntoResponse for Bytes);
some_impl!(Vec<u8>; IntoResponse for BytesMut);
some_impl!(Vec<u8>; IntoResponse for Box<[u8]>);

some_impl!([u8; N]; <const N: usize> IntoResponse for [u8; N]);
some_impl!([u8; N]; <const N: usize> IntoResponse for &'static [u8; N]);

impl<T, E> IntoResponse for Result<T, E>
where
    T: IntoResponse,
    E: ResponseError,
{
    type Error = Either<T::Error, E>;

    fn into_response(self) -> Result<Response, Self::Error> {
        match self {
            Ok(t) => t.into_response().map_err(Either::Left),
            Err(e) => Err(Either::Right(e)),
        }
    }

    fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        T::responses(components)
    }
}
