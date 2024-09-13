use std::{collections::HashSet, error::Error, fmt, str::Utf8Error, sync::Arc};

use http::{
    header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    HeaderName, StatusCode,
};
use http_body_util::LengthLimitError;
use predawn_core::media_type::MediaType;
pub use predawn_core::response_error::*;

use crate::{
    extract::multipart::Multipart,
    payload::{Form, Json},
    response::ToHeaderValue,
};

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
    #[error("failed to parse `{key}` with value {value:?} to a `{expected_type}`")]
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
pub enum DeserializeJsonError {
    #[error("input that is not syntactically valid JSON: {0}")]
    SyntaxError(#[source] serde_path_to_error::Error<serde_json::Error>),
    #[error("input data that is semantically incorrect: {0}")]
    DataError(#[source] serde_path_to_error::Error<serde_json::Error>),
    #[error("unexpected end of the input data: {0}")]
    EofError(#[source] serde_path_to_error::Error<serde_json::Error>),
}

impl ResponseError for DeserializeJsonError {
    fn as_status(&self) -> StatusCode {
        match self {
            DeserializeJsonError::SyntaxError(_) => StatusCode::BAD_REQUEST,
            DeserializeJsonError::DataError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            DeserializeJsonError::EofError(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::BAD_REQUEST, StatusCode::UNPROCESSABLE_ENTITY].into()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReadJsonError {
    #[error("expected request with `{}: {}`", CONTENT_TYPE, <Json<()> as MediaType>::MEDIA_TYPE)]
    InvalidJsonContentType,
    #[error("{0}")]
    ReadBytesError(#[from] ReadBytesError),
    #[error("{0}")]
    DeserializeJsonError(#[from] DeserializeJsonError),
}

impl ResponseError for ReadJsonError {
    fn as_status(&self) -> StatusCode {
        match self {
            ReadJsonError::InvalidJsonContentType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ReadJsonError::ReadBytesError(e) => e.as_status(),
            ReadJsonError::DeserializeJsonError(e) => e.as_status(),
        }
    }

    fn status_codes() -> HashSet<StatusCode> {
        let mut status_codes = ReadBytesError::status_codes();
        status_codes.extend(DeserializeJsonError::status_codes());
        status_codes.insert(StatusCode::UNSUPPORTED_MEDIA_TYPE);
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

#[derive(Debug, thiserror::Error)]
pub enum MultipartError {
    #[error("expected request with `{}: {}`", CONTENT_TYPE, <Multipart as MediaType>::MEDIA_TYPE)]
    InvalidMultipartContentType,

    #[error("{0}")]
    ByParseMultipart(#[source] multer::Error),

    #[error("failed to parse field `{name}`: {error}")]
    ByParseField {
        name: &'static str,
        #[source]
        error: multer::Error,
    },

    #[error("duplicate field `{name}`")]
    DuplicateField { name: &'static str },

    #[error("failed to parse field `{name}` with value {value:?} to a `{expected_type}`")]
    ParseErrorAtName {
        name: &'static str,
        value: Box<str>,
        expected_type: &'static str,
    },

    #[error("missing field `{name}`")]
    MissingField { name: &'static str },

    #[error("failed to deserialize field `{name}` as JSON, {error}")]
    DeserializeJson {
        name: &'static str,
        #[source]
        error: DeserializeJsonError,
    },

    #[error("missing file name for field `{name}`")]
    MissingFileName { name: &'static str },

    #[error("missing content type for field `{name}`")]
    MissingContentType { name: &'static str },

    #[error("incorrect number of fields for `{name}`: expected {expected} but actual {actual}")]
    IncorrectNumberOfFields {
        name: &'static str,
        expected: usize,
        actual: usize,
    },
}

impl ResponseError for MultipartError {
    fn as_status(&self) -> StatusCode {
        match self {
            MultipartError::InvalidMultipartContentType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            MultipartError::ByParseMultipart(e) => status_code_from_multer_error(e),
            MultipartError::ByParseField { error, .. } => status_code_from_multer_error(error),
            MultipartError::DuplicateField { .. }
            | MultipartError::ParseErrorAtName { .. }
            | MultipartError::MissingField { .. }
            | MultipartError::DeserializeJson { .. }
            | MultipartError::MissingFileName { .. }
            | MultipartError::MissingContentType { .. }
            | MultipartError::IncorrectNumberOfFields { .. } => StatusCode::BAD_REQUEST,
        }
    }

    fn status_codes() -> HashSet<StatusCode> {
        let mut status_codes: HashSet<StatusCode> = [
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            StatusCode::BAD_REQUEST,
            StatusCode::PAYLOAD_TOO_LARGE,
            StatusCode::INTERNAL_SERVER_ERROR,
        ]
        .into();

        status_codes.extend(DeserializeJsonError::status_codes());

        status_codes
    }
}

fn status_code_from_multer_error(err: &multer::Error) -> StatusCode {
    match err {
        multer::Error::UnknownField { .. }
        | multer::Error::IncompleteFieldData { .. }
        | multer::Error::IncompleteHeaders
        | multer::Error::ReadHeaderFailed(..)
        | multer::Error::DecodeHeaderName { .. }
        | multer::Error::DecodeContentType(..)
        | multer::Error::NoBoundary
        | multer::Error::DecodeHeaderValue { .. }
        | multer::Error::NoMultipart
        | multer::Error::IncompleteStream => StatusCode::BAD_REQUEST,
        multer::Error::FieldSizeExceeded { .. } | multer::Error::StreamSizeExceeded { .. } => {
            StatusCode::PAYLOAD_TOO_LARGE
        }
        multer::Error::StreamReadFailed(err) => {
            if let Some(err) = err.downcast_ref::<multer::Error>() {
                return status_code_from_multer_error(err);
            }

            if err.is::<LengthLimitError>() {
                return StatusCode::PAYLOAD_TOO_LARGE;
            }

            StatusCode::INTERNAL_SERVER_ERROR
        }
        multer::Error::LockFailure => StatusCode::INTERNAL_SERVER_ERROR,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid `{CONTENT_DISPOSITION}` header value: `{0}`")]
pub struct InvalidContentDisposition(pub Box<str>);

impl ResponseError for InvalidContentDisposition {
    fn as_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::INTERNAL_SERVER_ERROR].into()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TypedHeaderError {
    #[error("missing header `{name}`")]
    Missing { name: &'static HeaderName },
    #[error("failed to decode header `{name}`: {error}")]
    DecodeError {
        name: &'static HeaderName,
        #[source]
        error: headers::Error,
    },
}

impl ResponseError for TypedHeaderError {
    fn as_status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::BAD_REQUEST].into()
    }
}

#[derive(Debug, thiserror::Error)]
#[error(
    "invalid `{CONTENT_TYPE}` header value: expected to be one of {expected:?} but actually {actual:?}"
)]
pub struct InvalidContentType<const N: usize> {
    pub actual: Box<str>,
    pub expected: [&'static str; N],
}

impl<const N: usize> ResponseError for InvalidContentType<N> {
    fn as_status(&self) -> StatusCode {
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::UNSUPPORTED_MEDIA_TYPE].into()
    }
}

#[derive(Debug)]
enum Kind {
    Error,
    None,
}

#[derive(Debug)]
pub struct InvalidHeaderValue {
    name: &'static str,
    value: Box<str>,
    type_name: &'static str,
    kind: Kind,
}

impl InvalidHeaderValue {
    pub fn error<T: ToHeaderValue>(name: &'static str, value: &T) -> Self {
        Self::new(name, value, Kind::Error)
    }

    pub fn none<T: ToHeaderValue>(name: &'static str, value: &T) -> Self {
        Self::new(name, value, Kind::None)
    }

    fn new<T: ToHeaderValue>(name: &'static str, value: &T, kind: Kind) -> Self {
        fn inner(
            name: &'static str,
            value: Box<str>,
            type_name: &'static str,
            kind: Kind,
        ) -> InvalidHeaderValue {
            InvalidHeaderValue {
                name,
                value,
                type_name,
                kind,
            }
        }

        inner(
            name,
            format!("{:?}", value).into(),
            std::any::type_name::<T>(),
            kind,
        )
    }
}

impl fmt::Display for InvalidHeaderValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            Kind::Error => write!(
                f,
                "cannot get a valid `{name}` header value, convert `{ty}` value `{v}` to a `HeaderValue` got `Error`",
                name = self.name,
                ty = self.type_name,
                v = self.value
            ),
            Kind::None => write!(
                f,
                "cannot get a valid `{name}` header value, convert `{ty}` value `{v}` to a `HeaderValue` got `None`, but `<{ty} as ToSchema>::REQUIRED` is `true`",
                name = self.name,
                ty = self.type_name,
                v = self.value
            ),
        }
    }
}

impl Error for InvalidHeaderValue {}

impl ResponseError for InvalidHeaderValue {
    fn as_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::INTERNAL_SERVER_ERROR].into()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WebSocketError {
    #[error("request method must be `GET`")]
    MethodNotGet,
    #[error("`Connection` header does not contains `upgrade`")]
    ConnectionHeaderNotContainsUpgrade,
    #[error("`Upgrade` header does not equal `websocket`")]
    UpgradeHeaderNotEqualWebSocket,
    #[error("`Sec-WebSocket-Version` header does not equal `13`")]
    SecWebSocketVersionHeaderNotEqual13,
    #[error("`Sec-WebSocket-Key` header not present")]
    SecWebSocketKeyHeaderNotPresent,
    #[error("request couldn't be upgraded to a WebSocket connection")]
    ConnectionNotUpgradable,
}

impl ResponseError for WebSocketError {
    fn as_status(&self) -> StatusCode {
        match self {
            WebSocketError::MethodNotGet => StatusCode::METHOD_NOT_ALLOWED,
            WebSocketError::ConnectionHeaderNotContainsUpgrade
            | WebSocketError::UpgradeHeaderNotEqualWebSocket
            | WebSocketError::SecWebSocketVersionHeaderNotEqual13
            | WebSocketError::SecWebSocketKeyHeaderNotPresent => StatusCode::BAD_REQUEST,
            WebSocketError::ConnectionNotUpgradable => StatusCode::UPGRADE_REQUIRED,
        }
    }

    fn status_codes() -> HashSet<StatusCode> {
        [
            StatusCode::METHOD_NOT_ALLOWED,
            StatusCode::BAD_REQUEST,
            StatusCode::UPGRADE_REQUIRED,
        ]
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_content_type() {
        let err = InvalidContentType {
            actual: "application/json".into(),
            expected: ["text/plain", "text/html"],
        };

        assert_eq!(
            err.to_string(),
            "invalid `content-type` header value: expected to be one of [\"text/plain\", \"text/html\"] but actually \"application/json\""
        );
    }
}
