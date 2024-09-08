use std::{convert::Infallible, future::Future};

use bytes::Bytes;
use futures_util::FutureExt;
use http::{HeaderMap, Method, Uri, Version};
use http_body_util::{BodyExt, LengthLimitError};

use crate::{
    body::RequestBody,
    private::{ViaRequest, ViaRequestHead},
    request::{BodyLimit, Head, LocalAddr, OriginalUri, RemoteAddr},
    response_error::{ReadBytesError, ReadStringError, RequestBodyLimitError, ResponseError},
};

pub trait FromRequestHead<'a>: Sized {
    type Error: ResponseError;

    fn from_request_head(head: &'a Head) -> impl Future<Output = Result<Self, Self::Error>> + Send;
}

pub trait FromRequest<'a, M = ViaRequest>: Sized {
    type Error: ResponseError;

    fn from_request(
        head: &'a Head,
        body: RequestBody,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send;
}

impl<'a, T> FromRequest<'a, ViaRequestHead> for T
where
    T: FromRequestHead<'a>,
{
    type Error = T::Error;

    async fn from_request(head: &'a Head, _: RequestBody) -> Result<Self, Self::Error> {
        // TODO: remove boxed when https://github.com/rust-lang/rust/issues/100013 is resolved
        T::from_request_head(head).boxed().await
    }
}

impl<'a, T: FromRequestHead<'a>> FromRequestHead<'a> for Option<T> {
    type Error = Infallible;

    async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error> {
        // TODO: remove boxed when https://github.com/rust-lang/rust/issues/100013 is resolved
        Ok(T::from_request_head(head).boxed().await.ok())
    }
}

impl<'a, T: FromRequest<'a>> FromRequest<'a> for Option<T> {
    type Error = Infallible;

    async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        // TODO: remove boxed when https://github.com/rust-lang/rust/issues/100013 is resolved
        Ok(T::from_request(head, body).boxed().await.ok())
    }
}

impl<'a, T: FromRequestHead<'a>> FromRequestHead<'a> for Result<T, T::Error> {
    type Error = Infallible;

    async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error> {
        // TODO: remove boxed when https://github.com/rust-lang/rust/issues/100013 is resolved
        Ok(T::from_request_head(head).boxed().await)
    }
}

impl<'a, T: FromRequest<'a>> FromRequest<'a> for Result<T, T::Error> {
    type Error = Infallible;

    async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        // TODO: remove boxed when https://github.com/rust-lang/rust/issues/100013 is resolved
        Ok(T::from_request(head, body).boxed().await)
    }
}

impl<'a> FromRequest<'a> for RequestBody {
    type Error = Infallible;

    async fn from_request(_: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        Ok(body)
    }
}

impl<'a> FromRequest<'a> for Bytes {
    type Error = ReadBytesError;

    async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        match body.collect().await {
            Ok(collected) => Ok(collected.to_bytes()),
            Err(err) => match err.downcast::<LengthLimitError>() {
                Ok(_) => Err(ReadBytesError::RequestBodyLimitError(
                    RequestBodyLimitError {
                        actual: head.content_length(),
                        expected: head.body_limit.0,
                    },
                )),
                Err(err) => Err(ReadBytesError::UnknownBodyError(err)),
            },
        }
    }
}

impl<'a> FromRequest<'a> for Vec<u8> {
    type Error = ReadBytesError;

    async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        Ok(Bytes::from_request(head, body).await?.into())
    }
}

impl<'a> FromRequest<'a> for String {
    type Error = ReadStringError;

    async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        let bytes = Vec::<u8>::from_request(head, body)
            .await
            .map_err(ReadStringError::ReadBytes)?;

        let string = String::from_utf8(bytes).map_err(ReadStringError::InvalidUtf8)?;

        Ok(string)
    }
}

macro_rules! impl_from_request_head_for_cloneable {
    ($ty:ty; $($field:ident)?) => {
        impl<'a> FromRequestHead<'a> for &'a $ty {
            type Error = Infallible;

            async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error> {
                Ok(&head $(.$field)?)
            }
        }

        impl<'a> FromRequestHead<'a> for $ty {
            type Error = Infallible;

            async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error> {
                Ok(Clone::clone(&head $(.$field)?))
            }
        }
    };
}

macro_rules! impl_from_request_head_for_copyable {
    ($ty:ty; $($field:ident)?) => {
        impl<'a> FromRequestHead<'a> for &'a $ty {
            type Error = Infallible;

            async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error> {
                Ok(&head $(.$field)?)
            }
        }

        impl<'a> FromRequestHead<'a> for $ty {
            type Error = Infallible;

            async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error> {
                Ok(head $(.$field)?)
            }
        }
    };
}

impl_from_request_head_for_cloneable!(Head; );
impl_from_request_head_for_cloneable!(Uri; uri);
impl_from_request_head_for_cloneable!(Method; method);
impl_from_request_head_for_cloneable!(HeaderMap; headers);
impl_from_request_head_for_cloneable!(OriginalUri; original_uri);

impl_from_request_head_for_copyable!(Version; version);
impl_from_request_head_for_copyable!(LocalAddr; local_addr);
impl_from_request_head_for_copyable!(RemoteAddr; remote_addr);
impl_from_request_head_for_copyable!(BodyLimit; body_limit);
