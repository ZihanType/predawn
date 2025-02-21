use std::convert::Infallible;

use bytes::Bytes;
use http::{HeaderMap, Method, Uri, Version};
use http_body_util::BodyExt;
use snafu::{IntoError, ResultExt};

use crate::{
    body::RequestBody,
    private::{ViaRequest, ViaRequestHead},
    request::{BodyLimit, Head, LocalAddr, OriginalUri, RemoteAddr},
    response_error::{
        InvalidUtf8Snafu, LengthLimitSnafu, ReadBytesError, ReadBytesSnafu, ReadStringError,
        ResponseError, read_bytes_error,
    },
};

pub trait FromRequestHead: Sized {
    type Error: ResponseError;

    fn from_request_head(head: &mut Head)
    -> impl Future<Output = Result<Self, Self::Error>> + Send;
}

pub trait FromRequest<M = ViaRequest>: Sized {
    type Error: ResponseError;

    fn from_request(
        head: &mut Head,
        body: RequestBody,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send;
}

pub trait OptionalFromRequestHead: Sized {
    type Error: ResponseError;

    fn from_request_head(
        head: &mut Head,
    ) -> impl Future<Output = Result<Option<Self>, Self::Error>> + Send;
}

pub trait OptionalFromRequest: Sized {
    type Error: ResponseError;

    fn from_request(
        head: &mut Head,
        body: RequestBody,
    ) -> impl Future<Output = Result<Option<Self>, Self::Error>> + Send;
}

impl<T> FromRequest<ViaRequestHead> for T
where
    T: FromRequestHead,
{
    type Error = T::Error;

    async fn from_request(head: &mut Head, _: RequestBody) -> Result<Self, Self::Error> {
        T::from_request_head(head).await
    }
}

impl<T: OptionalFromRequestHead> FromRequestHead for Option<T> {
    type Error = T::Error;

    async fn from_request_head(head: &mut Head) -> Result<Self, Self::Error> {
        T::from_request_head(head).await
    }
}

impl<T: OptionalFromRequest> FromRequest for Option<T> {
    type Error = T::Error;

    async fn from_request(head: &mut Head, body: RequestBody) -> Result<Self, Self::Error> {
        T::from_request(head, body).await
    }
}

impl<T: FromRequestHead> FromRequestHead for Result<T, T::Error> {
    type Error = Infallible;

    async fn from_request_head(head: &mut Head) -> Result<Self, Self::Error> {
        Ok(T::from_request_head(head).await)
    }
}

impl<T: FromRequest> FromRequest for Result<T, T::Error> {
    type Error = Infallible;

    async fn from_request(head: &mut Head, body: RequestBody) -> Result<Self, Self::Error> {
        Ok(T::from_request(head, body).await)
    }
}

impl FromRequest for RequestBody {
    type Error = Infallible;

    async fn from_request(_: &mut Head, body: RequestBody) -> Result<Self, Self::Error> {
        Ok(body)
    }
}

impl FromRequest for Bytes {
    type Error = ReadBytesError;

    async fn from_request(head: &mut Head, body: RequestBody) -> Result<Self, Self::Error> {
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

impl FromRequest for Vec<u8> {
    type Error = ReadBytesError;

    async fn from_request(head: &mut Head, body: RequestBody) -> Result<Self, Self::Error> {
        Ok(Bytes::from_request(head, body).await?.into())
    }
}

impl FromRequest for String {
    type Error = ReadStringError;

    async fn from_request(head: &mut Head, body: RequestBody) -> Result<Self, Self::Error> {
        let bytes = Vec::<u8>::from_request(head, body)
            .await
            .context(ReadBytesSnafu)?;

        let string = String::from_utf8(bytes).context(InvalidUtf8Snafu)?;

        Ok(string)
    }
}

macro_rules! some_impl {
    ($ty:ty; $($field:ident)?) => {
        impl FromRequestHead for $ty {
            type Error = Infallible;

            async fn from_request_head(head: &mut Head) -> Result<Self, Self::Error> {
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
