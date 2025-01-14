use std::{convert::Infallible, future::Future};

use bytes::Bytes;
use futures_util::FutureExt;
use http::{HeaderMap, Method, Uri, Version};
use http_body_util::BodyExt;
use snafu::{IntoError, ResultExt};

use crate::{
    body::RequestBody,
    private::{ViaRequest, ViaRequestHead},
    request::{BodyLimit, Head, LocalAddr, OriginalUri, RemoteAddr},
    response_error::{
        read_bytes_error, InvalidUtf8Snafu, LengthLimitSnafu, ReadBytesError, ReadBytesSnafu,
        ReadStringError, ResponseError,
    },
};

pub trait FromRequestHead<'a>: Sized {
    type Error: ResponseError;

    fn from_request_head(
        head: &'a mut Head,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send;
}

pub trait FromRequest<'a, M = ViaRequest>: Sized {
    type Error: ResponseError;

    fn from_request(
        head: &'a mut Head,
        body: RequestBody,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send;
}

pub trait OptionalFromRequestHead<'a>: Sized {
    type Error: ResponseError;

    fn from_request_head(
        head: &'a mut Head,
    ) -> impl Future<Output = Result<Option<Self>, Self::Error>> + Send;
}

pub trait OptionalFromRequest<'a, M = ViaRequest>: Sized {
    type Error: ResponseError;

    fn from_request(
        head: &'a mut Head,
        body: RequestBody,
    ) -> impl Future<Output = Result<Option<Self>, Self::Error>> + Send;
}

impl<'a, T> FromRequest<'a, ViaRequestHead> for T
where
    T: FromRequestHead<'a>,
{
    type Error = T::Error;

    async fn from_request(head: &'a mut Head, _: RequestBody) -> Result<Self, Self::Error> {
        // TODO: remove boxed when https://github.com/rust-lang/rust/issues/100013 is resolved
        T::from_request_head(head).boxed().await
    }
}

impl<'a, T: OptionalFromRequestHead<'a>> FromRequestHead<'a> for Option<T> {
    type Error = T::Error;

    async fn from_request_head(head: &'a mut Head) -> Result<Self, Self::Error> {
        // TODO: remove boxed when https://github.com/rust-lang/rust/issues/100013 is resolved
        T::from_request_head(head).boxed().await
    }
}

impl<'a, T: OptionalFromRequest<'a>> FromRequest<'a> for Option<T> {
    type Error = T::Error;

    async fn from_request(head: &'a mut Head, body: RequestBody) -> Result<Self, Self::Error> {
        // TODO: remove boxed when https://github.com/rust-lang/rust/issues/100013 is resolved
        T::from_request(head, body).boxed().await
    }
}

impl<'a, T: FromRequestHead<'a>> FromRequestHead<'a> for Result<T, T::Error> {
    type Error = Infallible;

    async fn from_request_head(head: &'a mut Head) -> Result<Self, Self::Error> {
        // TODO: remove boxed when https://github.com/rust-lang/rust/issues/100013 is resolved
        Ok(T::from_request_head(head).boxed().await)
    }
}

impl<'a, T: FromRequest<'a>> FromRequest<'a> for Result<T, T::Error> {
    type Error = Infallible;

    async fn from_request(head: &'a mut Head, body: RequestBody) -> Result<Self, Self::Error> {
        // TODO: remove boxed when https://github.com/rust-lang/rust/issues/100013 is resolved
        Ok(T::from_request(head, body).boxed().await)
    }
}

impl<'a> FromRequest<'a> for RequestBody {
    type Error = Infallible;

    async fn from_request(_: &'a mut Head, body: RequestBody) -> Result<Self, Self::Error> {
        Ok(body)
    }
}

impl<'a> FromRequest<'a> for Bytes {
    type Error = ReadBytesError;

    async fn from_request(head: &'a mut Head, body: RequestBody) -> Result<Self, Self::Error> {
        let limit = head.body_limit().0;

        match body.collect().await {
            Ok(collected) => Ok(collected.to_bytes()),
            Err(err) => match err.downcast::<http_body_util::LengthLimitError>() {
                Ok(_) => {
                    let err = LengthLimitSnafu { limit }.build();
                    Err(read_bytes_error::LengthLimitSnafu.into_error(err))
                }
                Err(err) => Err(read_bytes_error::UnknownBodySnafu.into_error(err)),
            },
        }
    }
}

impl<'a> FromRequest<'a> for Vec<u8> {
    type Error = ReadBytesError;

    async fn from_request(head: &'a mut Head, body: RequestBody) -> Result<Self, Self::Error> {
        Ok(Bytes::from_request(head, body).await?.into())
    }
}

impl<'a> FromRequest<'a> for String {
    type Error = ReadStringError;

    async fn from_request(head: &'a mut Head, body: RequestBody) -> Result<Self, Self::Error> {
        let bytes = Vec::<u8>::from_request(head, body)
            .await
            .context(ReadBytesSnafu)?;

        let string = String::from_utf8(bytes).context(InvalidUtf8Snafu)?;

        Ok(string)
    }
}

macro_rules! some_impl {
    ($ty:ty; $($field:ident)?) => {
        impl<'a> FromRequestHead<'a> for $ty {
            type Error = Infallible;

            async fn from_request_head(head: &'a mut Head) -> Result<Self, Self::Error> {
                Ok(Clone::clone(&head $(.$field)?))
            }
        }
    };
}

some_impl!(Head; );
some_impl!(Uri; uri);
some_impl!(Method; method);
some_impl!(HeaderMap; headers);
some_impl!(OriginalUri; original_uri);
some_impl!(Version; version);
some_impl!(LocalAddr; local_addr);
some_impl!(RemoteAddr; remote_addr);
some_impl!(BodyLimit; body_limit);
