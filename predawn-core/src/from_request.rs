use std::{collections::HashSet, convert::Infallible, string::FromUtf8Error};

use async_trait::async_trait;
use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode, Uri, Version};
use http_body_util::{BodyExt, LengthLimitError};

use crate::{
    body::RequestBody,
    error::BoxError,
    media_type::MultiRequestMediaType,
    openapi::{self, Components, Parameter},
    request::{Head, LocalAddr, OriginalUri, RemoteAddr},
    response_error::ResponseError,
};

mod private {
    #[derive(Debug, Clone, Copy)]
    pub enum ViaHead {}

    #[derive(Debug, Clone, Copy)]
    pub enum ViaRequest {}
}

// TODO: remove #[async_trait] when https://github.com/rust-lang/rust/issues/100013 is resolved
#[async_trait]
pub trait FromRequestHead<'a>: Sized {
    type Error: ResponseError;

    async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error>;

    fn parameters(components: &mut Components) -> Option<Vec<Parameter>>;
}

// TODO: remove #[async_trait] when https://github.com/rust-lang/rust/issues/100013 is resolved
#[async_trait]
pub trait FromRequest<'a, M = private::ViaRequest>: Sized {
    type Error: ResponseError;

    async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error>;

    fn parameters(components: &mut Components) -> Option<Vec<Parameter>>;

    fn request_body(components: &mut Components) -> Option<openapi::RequestBody>;
}

#[async_trait]
impl<'a, T: FromRequestHead<'a>> FromRequest<'a, private::ViaHead> for T {
    type Error = T::Error;

    async fn from_request(head: &'a Head, _: RequestBody) -> Result<Self, Self::Error> {
        T::from_request_head(head).await
    }

    fn parameters(components: &mut Components) -> Option<Vec<Parameter>> {
        T::parameters(components)
    }

    fn request_body(_: &mut Components) -> Option<openapi::RequestBody> {
        None
    }
}

macro_rules! optional_parameters {
    ($ty:ty) => {
        fn parameters(components: &mut Components) -> Option<Vec<Parameter>> {
            let mut parameters = <$ty>::parameters(components)?;

            parameters.iter_mut().for_each(|parameter| match parameter {
                Parameter::Query { parameter_data, .. } => parameter_data.required = false,
                Parameter::Header { parameter_data, .. } => parameter_data.required = false,
                Parameter::Path { parameter_data, .. } => parameter_data.required = false,
                Parameter::Cookie { parameter_data, .. } => parameter_data.required = false,
            });

            Some(parameters)
        }
    };
}

#[async_trait]
impl<'a, T: FromRequestHead<'a>> FromRequestHead<'a> for Option<T> {
    type Error = Infallible;

    optional_parameters!(T);

    async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error> {
        Ok(T::from_request_head(head).await.ok())
    }
}

#[async_trait]
impl<'a, T: FromRequest<'a>> FromRequest<'a> for Option<T> {
    type Error = Infallible;

    optional_parameters!(T);

    async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        Ok(T::from_request(head, body).await.ok())
    }

    fn request_body(components: &mut Components) -> Option<openapi::RequestBody> {
        let mut request_body = T::request_body(components)?;
        request_body.required = false;
        Some(request_body)
    }
}

#[async_trait]
impl<'a, T: FromRequestHead<'a>> FromRequestHead<'a> for Result<T, T::Error> {
    type Error = Infallible;

    async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error> {
        Ok(T::from_request_head(head).await)
    }

    fn parameters(components: &mut Components) -> Option<Vec<Parameter>> {
        T::parameters(components)
    }
}

#[async_trait]
impl<'a, T: FromRequest<'a>> FromRequest<'a> for Result<T, T::Error> {
    type Error = Infallible;

    async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        Ok(T::from_request(head, body).await)
    }

    fn parameters(components: &mut Components) -> Option<Vec<Parameter>> {
        T::parameters(components)
    }

    fn request_body(components: &mut Components) -> Option<openapi::RequestBody> {
        T::request_body(components)
    }
}

#[async_trait]
impl<'a> FromRequest<'a> for RequestBody {
    type Error = Infallible;

    async fn from_request(_: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        Ok(body)
    }

    fn parameters(_: &mut Components) -> Option<Vec<Parameter>> {
        None
    }

    fn request_body(components: &mut Components) -> Option<openapi::RequestBody> {
        Bytes::request_body(components)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReadBytesError {
    #[error("failed to read bytes from request body: {0}")]
    LengthLimitError(#[from] LengthLimitError),
    #[error("failed to read bytes from request body: {0}")]
    UnknownBodyError(#[from] BoxError),
}

impl ResponseError for ReadBytesError {
    fn as_status(&self) -> StatusCode {
        match self {
            ReadBytesError::LengthLimitError(_) => StatusCode::PAYLOAD_TOO_LARGE,
            ReadBytesError::UnknownBodyError(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::PAYLOAD_TOO_LARGE, StatusCode::BAD_REQUEST].into()
    }
}

#[async_trait]
impl<'a> FromRequest<'a> for Bytes {
    type Error = ReadBytesError;

    async fn from_request(_: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        match body.collect().await {
            Ok(collected) => Ok(collected.to_bytes()),
            Err(err) => match err.downcast::<LengthLimitError>() {
                Ok(err) => Err(ReadBytesError::LengthLimitError(*err)),
                Err(err) => Err(ReadBytesError::UnknownBodyError(err)),
            },
        }
    }

    fn parameters(_: &mut Components) -> Option<Vec<Parameter>> {
        None
    }

    fn request_body(components: &mut Components) -> Option<openapi::RequestBody> {
        Some(openapi::RequestBody {
            description: Some("Extract binary from request body".to_owned()),
            content: <Bytes as MultiRequestMediaType>::content(components),
            required: true,
            ..Default::default()
        })
    }
}

#[async_trait]
impl<'a> FromRequest<'a> for Vec<u8> {
    type Error = ReadBytesError;

    async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        Ok(Bytes::from_request(head, body).await?.into())
    }

    fn parameters(_: &mut Components) -> Option<Vec<Parameter>> {
        None
    }

    fn request_body(components: &mut Components) -> Option<openapi::RequestBody> {
        Bytes::request_body(components)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReadStringError {
    #[error("{0}")]
    ReadBytes(#[from] ReadBytesError),
    #[error("failed to convert bytes to string: {0}")]
    InvalidUtf8(#[from] FromUtf8Error),
}

impl ResponseError for ReadStringError {
    fn as_status(&self) -> StatusCode {
        match self {
            ReadStringError::ReadBytes(err) => err.as_status(),
            ReadStringError::InvalidUtf8(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn status_codes() -> HashSet<StatusCode> {
        let mut status_codes = ReadBytesError::status_codes();
        status_codes.insert(StatusCode::BAD_REQUEST);
        status_codes
    }
}

#[async_trait]
impl<'a> FromRequest<'a> for String {
    type Error = ReadStringError;

    async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        let bytes = Vec::<u8>::from_request(head, body)
            .await
            .map_err(ReadStringError::ReadBytes)?;

        let string = String::from_utf8(bytes).map_err(ReadStringError::InvalidUtf8)?;

        Ok(string)
    }

    fn parameters(_: &mut Components) -> Option<Vec<Parameter>> {
        None
    }

    fn request_body(components: &mut Components) -> Option<openapi::RequestBody> {
        Some(openapi::RequestBody {
            description: Some("Extract text from request body".to_owned()),
            content: <String as MultiRequestMediaType>::content(components),
            required: true,
            ..Default::default()
        })
    }
}

macro_rules! impl_from_request_head_for_cloneable {
    ($ty:ty; $($field:ident)?) => {
        #[async_trait]
        impl<'a> FromRequestHead<'a> for &'a $ty {
            type Error = Infallible;

            async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error> {
                Ok(&head $(.$field)?)
            }

            fn parameters(_: &mut Components) -> Option<Vec<Parameter>> {
                None
            }
        }

        #[async_trait]
        impl<'a> FromRequestHead<'a> for $ty {
            type Error = Infallible;

            async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error> {
                Ok(Clone::clone(&head $(.$field)?))
            }

            fn parameters(_: &mut Components) -> Option<Vec<Parameter>> {
                None
            }
        }
    };
}

macro_rules! impl_from_request_head_for_copyable {
    ($ty:ty; $($field:ident)?) => {
        #[async_trait]
        impl<'a> FromRequestHead<'a> for $ty {
            type Error = Infallible;

            async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error> {
                Ok(head $(.$field)?)
            }

            fn parameters(_: &mut Components) -> Option<Vec<Parameter>> {
                None
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
