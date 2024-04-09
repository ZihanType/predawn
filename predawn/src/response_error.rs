use std::{collections::HashSet, str::Utf8Error, sync::Arc};

use http::{header::CONTENT_TYPE, StatusCode};
use predawn_core::media_type::MediaType;
pub use predawn_core::response_error::*;

use crate::payload::{Form, Json};

#[derive(Debug, thiserror::Error)]
#[error("method not allowed")]
pub struct MethodNotAllowedError;

impl ResponseError for MethodNotAllowedError {
    fn as_status(&self) -> StatusCode {
        StatusCode::METHOD_NOT_ALLOWED
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::METHOD_NOT_ALLOWED].into()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct MatchError(#[from] pub matchit::MatchError);

impl ResponseError for MatchError {
    fn as_status(&self) -> StatusCode {
        match self.0 {
            matchit::MatchError::NotFound => StatusCode::NOT_FOUND,
        }
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::NOT_FOUND].into()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed to deserialize query data: {0}")]
pub struct QueryError(#[from] pub serde_html_form::de::Error);

impl ResponseError for QueryError {
    fn as_status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::BAD_REQUEST].into()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("no paths parameters found for matched route")]
    MissingPathParams,

    /// The URI contained the wrong number of parameters.
    #[error("wrong number of parameters: expected {expected} but actual {actual}")]
    WrongNumberOfParameters {
        /// The number of actual parameters in the URI.
        actual: usize,
        /// The number of expected parameters.
        expected: usize,
    },

    /// Failed to parse the value at a specific key into the expected type.
    ///
    /// This variant is used when deserializing into types that have named fields, such as structs.
    #[error("failed to parse `{key}` with value `{value:?}` to a `{expected_type}`")]
    ParseErrorAtKey {
        /// The key at which the value was located.
        key: Arc<str>,
        /// The value from the URI.
        value: Arc<str>,
        /// The expected type of the value.
        expected_type: &'static str,
    },

    /// A parameter contained text that, once percent decoded, wasn't valid UTF-8.
    #[error("{error} in `{key}`")]
    InvalidUtf8InPathParam {
        /// The key at which the invalid value was located.
        key: Arc<str>,
        error: Utf8Error,
    },

    /// Tried to serialize into an unsupported type such as nested maps.
    ///
    /// This error kind is caused by programmer errors and thus gets converted into a `500 Internal
    /// Server Error` response.
    #[error("unsupported type: {name}")]
    UnsupportedType {
        /// The name of the unsupported type.
        name: &'static str,
    },

    /// Catch-all variant for errors that don't fit any other variant.
    #[error("{0}")]
    Message(String),
}

impl ResponseError for PathError {
    fn as_status(&self) -> StatusCode {
        match self {
            PathError::MissingPathParams
            | PathError::WrongNumberOfParameters { .. }
            | PathError::UnsupportedType { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            PathError::ParseErrorAtKey { .. }
            | PathError::InvalidUtf8InPathParam { .. }
            | PathError::Message(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::INTERNAL_SERVER_ERROR, StatusCode::BAD_REQUEST].into()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReadFormError {
    #[error("{0}")]
    ReadBytesError(#[from] ReadBytesError),
    #[error("expected request with `{}: {}`", CONTENT_TYPE, <Form<()> as MediaType>::MEDIA_TYPE)]
    InvalidFormContentType,
    #[error("failed to deserialize form data: {0}")]
    FormDeserializeError(#[from] serde_html_form::de::Error),
}

impl ResponseError for ReadFormError {
    fn as_status(&self) -> StatusCode {
        match self {
            ReadFormError::ReadBytesError(e) => e.as_status(),
            ReadFormError::InvalidFormContentType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ReadFormError::FormDeserializeError(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn status_codes() -> HashSet<StatusCode> {
        let mut status_codes = ReadBytesError::status_codes();
        status_codes.insert(StatusCode::UNSUPPORTED_MEDIA_TYPE);
        status_codes.insert(StatusCode::BAD_REQUEST);
        status_codes
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed to serialize form data: {0}")]
pub struct WriteFormError(#[from] pub serde_html_form::ser::Error);

impl ResponseError for WriteFormError {
    fn as_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::INTERNAL_SERVER_ERROR].into()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReadJsonError {
    #[error("expected request with `{}: {}`", CONTENT_TYPE, <Json<()> as MediaType>::MEDIA_TYPE)]
    InvalidJsonContentType,
    #[error("{0}")]
    ReadBytesError(#[from] ReadBytesError),
    #[error("input data that is semantically incorrect: {0}")]
    JsonDataError(#[source] serde_path_to_error::Error<serde_json::Error>),
    #[error("input that is not syntactically valid JSON: {0}")]
    JsonSyntaxError(#[source] serde_path_to_error::Error<serde_json::Error>),
}

impl ResponseError for ReadJsonError {
    fn as_status(&self) -> StatusCode {
        match self {
            ReadJsonError::InvalidJsonContentType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ReadJsonError::ReadBytesError(e) => e.as_status(),
            ReadJsonError::JsonDataError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ReadJsonError::JsonSyntaxError(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn status_codes() -> HashSet<StatusCode> {
        let mut status_codes = ReadBytesError::status_codes();
        status_codes.insert(StatusCode::UNSUPPORTED_MEDIA_TYPE);
        status_codes.insert(StatusCode::UNPROCESSABLE_ENTITY);
        status_codes.insert(StatusCode::BAD_REQUEST);
        status_codes
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed to serialize response as JSON: {0}")]
pub struct WriteJsonError(#[from] pub serde_json::Error);

impl ResponseError for WriteJsonError {
    fn as_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::INTERNAL_SERVER_ERROR].into()
    }
}
