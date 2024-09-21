use std::{
    collections::{BTreeMap, BTreeSet},
    convert::Infallible,
    error::Error,
    fmt,
    string::FromUtf8Error,
};

use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use mime::TEXT_PLAIN_UTF_8;
use snafu::Snafu;

use crate::{
    error::BoxError,
    error_stack::ErrorStack,
    location::Location,
    media_type::MultiResponseMediaType,
    openapi::{self, Schema},
    response::Response,
};

pub trait ResponseError: Error + Send + Sync + Sized + 'static {
    fn as_status(&self) -> StatusCode;

    fn status_codes(codes: &mut BTreeSet<StatusCode>);

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
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> BTreeMap<StatusCode, openapi::Response> {
        let mut codes = BTreeSet::new();

        Self::status_codes(&mut codes);

        codes
            .into_iter()
            .map(|status| {
                (
                    status,
                    openapi::Response {
                        description: status.canonical_reason().unwrap_or_default().to_string(),
                        content: <String as MultiResponseMediaType>::content(
                            schemas,
                            schemas_in_progress,
                        ),
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

    fn error_stack(&self, stack: &mut ErrorStack);
}

impl ResponseError for Infallible {
    fn as_status(&self) -> StatusCode {
        match *self {}
    }

    fn status_codes(_: &mut BTreeSet<StatusCode>) {}

    fn as_response(&self) -> Response {
        match *self {}
    }

    fn responses(
        _: &mut BTreeMap<String, Schema>,
        _: &mut Vec<String>,
    ) -> BTreeMap<StatusCode, openapi::Response> {
        BTreeMap::new()
    }

    fn error_stack(&self, _: &mut ErrorStack) {
        match *self {}
    }
}

#[derive(Debug)]
pub struct RequestBodyLimitError {
    pub location: Location,
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

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::PAYLOAD_TOO_LARGE);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        stack.push(self, &self.location);
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ReadBytesError {
    #[snafu(display("{source}"))]
    RequestBodyLimitError {
        #[snafu(implicit)]
        location: Location,
        source: RequestBodyLimitError,
    },
    #[snafu(display("failed to read bytes from request body"))]
    UnknownBodyError {
        #[snafu(implicit)]
        location: Location,
        source: BoxError,
    },
}

impl ResponseError for ReadBytesError {
    fn as_status(&self) -> StatusCode {
        match self {
            ReadBytesError::RequestBodyLimitError { source, .. } => source.as_status(),
            ReadBytesError::UnknownBodyError { .. } => StatusCode::BAD_REQUEST,
        }
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        RequestBodyLimitError::status_codes(codes);
        codes.insert(StatusCode::BAD_REQUEST);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        match self {
            ReadBytesError::RequestBodyLimitError { location, source } => {
                stack.push(self, location);
                source.error_stack(stack);
            }
            ReadBytesError::UnknownBodyError { location, source } => {
                stack.push(self, location);
                stack.push_without_location(source);
            }
        }
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ReadStringError {
    #[snafu(display("{source}"))]
    ReadBytes {
        #[snafu(implicit)]
        location: Location,
        source: ReadBytesError,
    },
    #[snafu(display("{source}"))]
    InvalidUtf8 {
        #[snafu(implicit)]
        location: Location,
        source: FromUtf8Error,
    },
}

impl ResponseError for ReadStringError {
    fn as_status(&self) -> StatusCode {
        match self {
            ReadStringError::ReadBytes { source, .. } => source.as_status(),
            ReadStringError::InvalidUtf8 { .. } => StatusCode::BAD_REQUEST,
        }
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        ReadBytesError::status_codes(codes);
        codes.insert(StatusCode::BAD_REQUEST);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        match self {
            ReadStringError::ReadBytes { location, source } => {
                stack.push(self, location);
                source.error_stack(stack);
            }
            ReadStringError::InvalidUtf8 { location, source } => {
                stack.push(self, location);
                stack.push_without_location(source);
            }
        }
    }
}
