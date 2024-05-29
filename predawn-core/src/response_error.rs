use std::{
    collections::{BTreeMap, HashSet},
    convert::Infallible,
    error::Error,
    fmt,
    string::FromUtf8Error,
};

use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use indexmap::IndexMap;
use mime::TEXT_PLAIN_UTF_8;
use openapiv3::{ReferenceOr, Schema};

use crate::{error::BoxError, media_type::MultiResponseMediaType, openapi, response::Response};

pub trait ResponseError: Error + Send + Sync + Sized + 'static {
    fn as_status(&self) -> StatusCode;

    fn status_codes() -> HashSet<StatusCode>;

    fn as_response(&self) -> Response {
        Response::builder()
            .status(self.as_status())
            .header(
                CONTENT_TYPE,
                HeaderValue::from_static(TEXT_PLAIN_UTF_8.as_ref()),
            )
            .body(self.to_string().into())
            .unwrap()
    }

    fn responses(
        schemas: &mut IndexMap<String, ReferenceOr<Schema>>,
    ) -> BTreeMap<StatusCode, openapi::Response> {
        Self::status_codes()
            .into_iter()
            .map(|status| {
                (
                    status,
                    openapi::Response {
                        description: status.canonical_reason().unwrap_or_default().to_string(),
                        content: <String as MultiResponseMediaType>::content(schemas),
                        ..Default::default()
                    },
                )
            })
            .collect()
    }

    #[doc(hidden)]
    fn inner(self) -> BoxError {
        Box::new(self)
    }

    #[doc(hidden)]
    fn wrappers(&self, type_names: &mut Vec<&'static str>) {
        type_names.push(std::any::type_name::<Self>());
    }
}

impl ResponseError for Infallible {
    fn as_status(&self) -> StatusCode {
        match *self {}
    }

    fn status_codes() -> HashSet<StatusCode> {
        HashSet::new()
    }

    fn as_response(&self) -> Response {
        match *self {}
    }

    fn responses(
        _: &mut IndexMap<String, ReferenceOr<Schema>>,
    ) -> BTreeMap<StatusCode, openapi::Response> {
        BTreeMap::new()
    }
}

#[derive(Debug)]
pub struct RequestBodyLimitError {
    pub actual: Option<usize>,
    pub expected: usize,
}

impl fmt::Display for RequestBodyLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.actual {
            Some(actual) => {
                write!(
                    f,
                    "payload too large: expected `{}` but actual `{}`",
                    self.expected, actual
                )
            }
            None => {
                write!(
                    f,
                    "payload too large (no content length): expected `{}`",
                    self.expected
                )
            }
        }
    }
}

impl Error for RequestBodyLimitError {}

impl ResponseError for RequestBodyLimitError {
    fn as_status(&self) -> StatusCode {
        StatusCode::PAYLOAD_TOO_LARGE
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::PAYLOAD_TOO_LARGE].into()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReadBytesError {
    #[error("{0}")]
    RequestBodyLimitError(#[from] RequestBodyLimitError),
    #[error("failed to read bytes from request body: {0}")]
    UnknownBodyError(#[from] BoxError),
}

impl ResponseError for ReadBytesError {
    fn as_status(&self) -> StatusCode {
        match self {
            ReadBytesError::RequestBodyLimitError(e) => e.as_status(),
            ReadBytesError::UnknownBodyError(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn status_codes() -> HashSet<StatusCode> {
        let mut status_codes = RequestBodyLimitError::status_codes();
        status_codes.insert(StatusCode::BAD_REQUEST);
        status_codes
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
            ReadStringError::ReadBytes(e) => e.as_status(),
            ReadStringError::InvalidUtf8(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn status_codes() -> HashSet<StatusCode> {
        let mut status_codes = ReadBytesError::status_codes();
        status_codes.insert(StatusCode::BAD_REQUEST);
        status_codes
    }
}
