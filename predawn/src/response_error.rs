use std::{collections::BTreeSet, error::Error, fmt, str::Utf8Error, sync::Arc};

use http::{
    header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    HeaderName, StatusCode,
};
use http_body_util::LengthLimitError;
pub use predawn_core::response_error::*;
use predawn_core::{error_stack::ErrorStack, location::Location, media_type::MediaType};
use snafu::Snafu;

use crate::{
    extract::multipart::Multipart,
    payload::{Form, Json},
    response::ToHeaderValue,
};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[snafu(display("method not allowed"))]
pub struct MethodNotAllowedError {
    #[snafu(implicit)]
    location: Location,
}

impl ResponseError for MethodNotAllowedError {
    fn as_status(&self) -> StatusCode {
        StatusCode::METHOD_NOT_ALLOWED
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::METHOD_NOT_ALLOWED);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        stack.push(self, &self.location);
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[snafu(display("{source}"))]
pub struct MatchError {
    #[snafu(implicit)]
    location: Location,
    source: matchit::MatchError,
}

impl ResponseError for MatchError {
    fn as_status(&self) -> StatusCode {
        match self.source {
            matchit::MatchError::NotFound => StatusCode::NOT_FOUND,
        }
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::NOT_FOUND);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        stack.push(self, &self.location);
        stack.push_without_location(&self.source);
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[snafu(display("{source}"))]
pub struct QueryError {
    #[snafu(implicit)]
    location: Location,
    source: serde_path_to_error::Error<serde_html_form::de::Error>,
}

impl ResponseError for QueryError {
    fn as_status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::BAD_REQUEST);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        stack.push(self, &self.location);
        stack.push_without_location(&self.source);
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum PathError {
    #[snafu(display("no paths parameters found for matched route"))]
    MissingPathParams {
        #[snafu(implicit)]
        location: Location,
    },

    /// A parameter contained text that, once percent decoded, wasn't valid UTF-8.
    #[snafu(display("{error} in `{key}`"))]
    InvalidUtf8InPathParam {
        #[snafu(implicit)]
        location: Location,
        /// The key at which the invalid value was located.
        key: Arc<str>,
        error: Utf8Error,
        error_location: Location,
    },

    #[snafu(display("{source}"))]
    DeserializePathError {
        #[snafu(implicit)]
        location: Location,
        source: serde_path_to_error::Error<DeserializePathError>,
    },
}

impl ResponseError for PathError {
    fn as_status(&self) -> StatusCode {
        match self {
            PathError::MissingPathParams { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            PathError::InvalidUtf8InPathParam { .. } => StatusCode::BAD_REQUEST,
            PathError::DeserializePathError { source, .. } => source.inner().as_status(),
        }
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::INTERNAL_SERVER_ERROR);
        codes.insert(StatusCode::BAD_REQUEST);
        DeserializePathError::status_codes(codes);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        match self {
            PathError::MissingPathParams { location } => {
                stack.push(self, location);
            }
            PathError::InvalidUtf8InPathParam {
                location,
                error,
                error_location,
                ..
            } => {
                stack.push(self, location);
                stack.push(error, error_location);
            }
            PathError::DeserializePathError { location, source } => {
                stack.push(self, location);
                source.inner().error_stack(stack);
            }
        }
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum DeserializePathError {
    /// The URI contained the wrong number of parameters.
    #[snafu(display("wrong number of parameters: expected {expected} but actual {actual}"))]
    WrongNumberOfParameters {
        #[snafu(implicit)]
        location: Location,
        /// The number of actual parameters in the URI.
        actual: usize,
        /// The number of expected parameters.
        expected: usize,
    },

    /// Failed to parse the value at a specific key into the expected type.
    ///
    /// This variant is used when deserializing into types that have named fields, such as structs.
    #[snafu(display("failed to parse `{key}` with value {value:?} to a `{expected_type}`"))]
    ParseErrorAtKey {
        #[snafu(implicit)]
        location: Location,
        /// The key at which the value was located.
        key: Arc<str>,
        /// The value from the URI.
        value: Arc<str>,
        /// The expected type of the value.
        expected_type: &'static str,
    },

    /// Tried to serialize into an unsupported type such as nested maps.
    ///
    /// This error kind is caused by programmer errors and thus gets converted into a `500 Internal
    /// Server Error` response.
    #[snafu(display("unsupported type: {name}"))]
    UnsupportedType {
        #[snafu(implicit)]
        location: Location,
        /// The name of the unsupported type.
        name: &'static str,
    },

    /// Catch-all variant for errors that don't fit any other variant.
    #[snafu(display("{message}"))]
    Message {
        #[snafu(implicit)]
        location: Location,
        message: Box<str>,
    },
}

impl serde::de::Error for DeserializePathError {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        MessageSnafu {
            message: msg.to_string().into_boxed_str(),
        }
        .build()
    }
}

impl ResponseError for DeserializePathError {
    fn as_status(&self) -> StatusCode {
        match self {
            DeserializePathError::WrongNumberOfParameters { .. }
            | DeserializePathError::UnsupportedType { .. } => StatusCode::INTERNAL_SERVER_ERROR,

            DeserializePathError::ParseErrorAtKey { .. } | DeserializePathError::Message { .. } => {
                StatusCode::BAD_REQUEST
            }
        }
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::INTERNAL_SERVER_ERROR);
        codes.insert(StatusCode::BAD_REQUEST);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        match self {
            DeserializePathError::WrongNumberOfParameters { location, .. }
            | DeserializePathError::ParseErrorAtKey { location, .. }
            | DeserializePathError::UnsupportedType { location, .. }
            | DeserializePathError::Message { location, .. } => {
                stack.push(self, location);
            }
        }
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ReadFormError {
    #[snafu(display("{source}"))]
    ReadFormBytesError {
        #[snafu(implicit)]
        location: Location,
        source: ReadBytesError,
    },
    #[snafu(display("expected request with `{}: {}`", CONTENT_TYPE, <Form<()> as MediaType>::MEDIA_TYPE))]
    InvalidFormContentType {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("{source}"))]
    FormDeserializeError {
        #[snafu(implicit)]
        location: Location,
        source: serde_path_to_error::Error<serde_html_form::de::Error>,
    },
}

impl ResponseError for ReadFormError {
    fn as_status(&self) -> StatusCode {
        match self {
            ReadFormError::ReadFormBytesError { source, .. } => source.as_status(),
            ReadFormError::InvalidFormContentType { .. } => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ReadFormError::FormDeserializeError { .. } => StatusCode::BAD_REQUEST,
        }
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        ReadBytesError::status_codes(codes);
        codes.insert(StatusCode::UNSUPPORTED_MEDIA_TYPE);
        codes.insert(StatusCode::BAD_REQUEST);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        match self {
            ReadFormError::ReadFormBytesError { location, source } => {
                stack.push(self, location);
                source.error_stack(stack);
            }
            ReadFormError::InvalidFormContentType { location } => {
                stack.push(self, location);
            }
            ReadFormError::FormDeserializeError { location, source } => {
                stack.push(self, location);
                stack.push_without_location(source);
            }
        }
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[snafu(display("{source}"))]
pub struct WriteFormError {
    #[snafu(implicit)]
    location: Location,
    source: serde_path_to_error::Error<serde_html_form::ser::Error>,
}

impl ResponseError for WriteFormError {
    fn as_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::INTERNAL_SERVER_ERROR);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        stack.push(self, &self.location);
        stack.push_without_location(&self.source);
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum DeserializeJsonError {
    #[snafu(display("input that is not syntactically valid JSON"))]
    SyntaxError {
        #[snafu(implicit)]
        location: Location,
        source: serde_path_to_error::Error<serde_json::Error>,
    },
    #[snafu(display("input data that is semantically incorrect"))]
    DataError {
        #[snafu(implicit)]
        location: Location,
        source: serde_path_to_error::Error<serde_json::Error>,
    },
    #[snafu(display("unexpected end of the input data"))]
    EofError {
        #[snafu(implicit)]
        location: Location,
        source: serde_path_to_error::Error<serde_json::Error>,
    },
}

impl ResponseError for DeserializeJsonError {
    fn as_status(&self) -> StatusCode {
        match self {
            DeserializeJsonError::SyntaxError { .. } => StatusCode::BAD_REQUEST,
            DeserializeJsonError::DataError { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            DeserializeJsonError::EofError { .. } => StatusCode::BAD_REQUEST,
        }
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::BAD_REQUEST);
        codes.insert(StatusCode::UNPROCESSABLE_ENTITY);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        match self {
            DeserializeJsonError::SyntaxError { location, source }
            | DeserializeJsonError::DataError { location, source }
            | DeserializeJsonError::EofError { location, source } => {
                stack.push(self, location);
                stack.push_without_location(source);
            }
        }
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ReadJsonError {
    #[snafu(display("expected request with `{}: {}`", CONTENT_TYPE, <Json<()> as MediaType>::MEDIA_TYPE))]
    InvalidJsonContentType {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("{source}"))]
    ReadJsonBytesError {
        #[snafu(implicit)]
        location: Location,
        source: ReadBytesError,
    },
    #[snafu(display("{source}"))]
    DeserializeJsonError {
        #[snafu(implicit)]
        location: Location,
        source: DeserializeJsonError,
    },
}

impl ResponseError for ReadJsonError {
    fn as_status(&self) -> StatusCode {
        match self {
            ReadJsonError::InvalidJsonContentType { .. } => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ReadJsonError::ReadJsonBytesError { source, .. } => source.as_status(),
            ReadJsonError::DeserializeJsonError { source, .. } => source.as_status(),
        }
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::UNSUPPORTED_MEDIA_TYPE);
        ReadBytesError::status_codes(codes);
        DeserializeJsonError::status_codes(codes);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        match self {
            ReadJsonError::InvalidJsonContentType { location } => {
                stack.push(self, location);
            }
            ReadJsonError::ReadJsonBytesError { location, source } => {
                stack.push(self, location);
                source.error_stack(stack);
            }
            ReadJsonError::DeserializeJsonError { location, source } => {
                stack.push(self, location);
                source.error_stack(stack);
            }
        }
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[snafu(display("failed to serialize response as JSON"))]
pub struct WriteJsonError {
    #[snafu(implicit)]
    location: Location,
    source: serde_path_to_error::Error<serde_json::Error>,
}

impl ResponseError for WriteJsonError {
    fn as_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::INTERNAL_SERVER_ERROR);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        stack.push(self, &self.location);
        stack.push_without_location(&self.source);
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum MultipartError {
    #[snafu(display("expected request with `{}: {}`", CONTENT_TYPE, <Multipart as MediaType>::MEDIA_TYPE))]
    InvalidMultipartContentType {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("{source}"))]
    ByParseMultipart {
        #[snafu(implicit)]
        location: Location,
        source: multer::Error,
    },

    #[snafu(display("failed to parse field `{name}`"))]
    ByParseField {
        #[snafu(implicit)]
        location: Location,
        name: &'static str,
        source: multer::Error,
    },

    #[snafu(display("duplicate field `{name}`"))]
    DuplicateField {
        #[snafu(implicit)]
        location: Location,
        name: &'static str,
    },

    #[snafu(display(
        "failed to parse field `{name}` with value {value:?} to a `{expected_type}`"
    ))]
    ParseErrorAtName {
        #[snafu(implicit)]
        location: Location,
        name: &'static str,
        value: Box<str>,
        expected_type: &'static str,
    },

    #[snafu(display("missing field `{name}`"))]
    MissingField {
        #[snafu(implicit)]
        location: Location,
        name: &'static str,
    },

    #[snafu(display("failed to deserialize field `{name}` as JSON"))]
    InvalidJsonField {
        #[snafu(implicit)]
        location: Location,
        name: &'static str,
        source: DeserializeJsonError,
    },

    #[snafu(display("missing file name for field `{name}`"))]
    MissingFileName {
        #[snafu(implicit)]
        location: Location,
        name: &'static str,
    },

    #[snafu(display("missing content type for field `{name}`"))]
    MissingContentType {
        #[snafu(implicit)]
        location: Location,
        name: &'static str,
    },

    #[snafu(display(
        "incorrect number of fields for `{name}`: expected {expected} but actual {actual}"
    ))]
    IncorrectNumberOfFields {
        #[snafu(implicit)]
        location: Location,
        name: &'static str,
        expected: usize,
        actual: usize,
    },
}

impl ResponseError for MultipartError {
    fn as_status(&self) -> StatusCode {
        match self {
            MultipartError::InvalidMultipartContentType { .. } => {
                StatusCode::UNSUPPORTED_MEDIA_TYPE
            }

            MultipartError::ByParseMultipart { source, .. }
            | MultipartError::ByParseField { source, .. } => status_code_from_multer_error(source),

            MultipartError::DuplicateField { .. }
            | MultipartError::ParseErrorAtName { .. }
            | MultipartError::MissingField { .. }
            | MultipartError::InvalidJsonField { .. }
            | MultipartError::MissingFileName { .. }
            | MultipartError::MissingContentType { .. }
            | MultipartError::IncorrectNumberOfFields { .. } => StatusCode::BAD_REQUEST,
        }
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        DeserializeJsonError::status_codes(codes);
        codes.insert(StatusCode::UNSUPPORTED_MEDIA_TYPE);
        codes.insert(StatusCode::BAD_REQUEST);
        codes.insert(StatusCode::PAYLOAD_TOO_LARGE);
        codes.insert(StatusCode::INTERNAL_SERVER_ERROR);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        match self {
            MultipartError::InvalidJsonField {
                location, source, ..
            } => {
                stack.push(self, location);
                source.error_stack(stack);
            }

            MultipartError::ByParseMultipart { location, source }
            | MultipartError::ByParseField {
                location, source, ..
            } => {
                stack.push(self, location);
                stack.push_without_location(source);
            }

            MultipartError::InvalidMultipartContentType { location, .. }
            | MultipartError::DuplicateField { location, .. }
            | MultipartError::ParseErrorAtName { location, .. }
            | MultipartError::MissingField { location, .. }
            | MultipartError::MissingFileName { location, .. }
            | MultipartError::MissingContentType { location, .. }
            | MultipartError::IncorrectNumberOfFields { location, .. } => {
                stack.push(self, location);
            }
        }
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

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[snafu(display("invalid `{CONTENT_DISPOSITION}` header value: `{value}`"))]
pub struct InvalidContentDisposition {
    #[snafu(implicit)]
    location: Location,
    value: Box<str>,
}

impl ResponseError for InvalidContentDisposition {
    fn as_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::INTERNAL_SERVER_ERROR);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        stack.push(self, &self.location);
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum TypedHeaderError {
    #[snafu(display("missing header `{name}`"))]
    Missing {
        #[snafu(implicit)]
        location: Location,
        name: &'static HeaderName,
    },
    #[snafu(display("failed to decode header `{name}`"))]
    DecodeError {
        #[snafu(implicit)]
        location: Location,
        name: &'static HeaderName,
        source: headers::Error,
    },
}

impl ResponseError for TypedHeaderError {
    fn as_status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::BAD_REQUEST);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        match self {
            TypedHeaderError::Missing { location, .. } => {
                stack.push(self, location);
            }
            TypedHeaderError::DecodeError {
                location, source, ..
            } => {
                stack.push(self, location);
                stack.push_without_location(source);
            }
        }
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[snafu(display(
    "invalid `{CONTENT_TYPE}` header value: expected to be one of {expected:?} but actually {actual:?}"
))]
pub struct InvalidContentType<const N: usize> {
    #[snafu(implicit)]
    pub location: Location,
    pub actual: Box<str>,
    pub expected: [&'static str; N],
}

impl<const N: usize> ResponseError for InvalidContentType<N> {
    fn as_status(&self) -> StatusCode {
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        stack.push(self, &self.location);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidHeaderValueKind {
    Error,
    None,
}

#[derive(Debug)]
pub struct InvalidHeaderValue {
    name: &'static str,
    value: Box<str>,
    type_name: &'static str,
    kind: InvalidHeaderValueKind,
    location: Location,
}

impl InvalidHeaderValue {
    #[track_caller]
    #[inline]
    pub fn error<T: ToHeaderValue>(name: &'static str, value: &T) -> Self {
        Self::new(name, value, InvalidHeaderValueKind::Error)
    }

    #[track_caller]
    #[inline]
    pub fn none<T: ToHeaderValue>(name: &'static str, value: &T) -> Self {
        Self::new(name, value, InvalidHeaderValueKind::None)
    }

    #[track_caller]
    #[inline]
    fn new<T: ToHeaderValue>(name: &'static str, value: &T, kind: InvalidHeaderValueKind) -> Self {
        fn inner(
            name: &'static str,
            value: Box<str>,
            type_name: &'static str,
            kind: InvalidHeaderValueKind,
        ) -> InvalidHeaderValue {
            InvalidHeaderValue {
                name,
                value,
                type_name,
                kind,
                location: Location::caller(),
            }
        }

        inner(
            name,
            format!("{:?}", value).into(),
            std::any::type_name::<T>(),
            kind,
        )
    }

    pub fn kind(&self) -> InvalidHeaderValueKind {
        self.kind
    }
}

impl fmt::Display for InvalidHeaderValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            InvalidHeaderValueKind::Error => write!(
                f,
                "cannot get a valid `{name}` header value, convert `{ty}` value `{v}` to a `HeaderValue` got `Error`",
                name = self.name,
                ty = self.type_name,
                v = self.value
            ),
            InvalidHeaderValueKind::None => write!(
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

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::INTERNAL_SERVER_ERROR);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        stack.push(self, &self.location);
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum WebSocketError {
    #[snafu(display("request method must be `GET`"))]
    MethodNotGet {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("`Connection` header does not contains `upgrade`"))]
    ConnectionHeaderNotContainsUpgrade {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("`Upgrade` header does not equal `websocket`"))]
    UpgradeHeaderNotEqualWebSocket {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("`Sec-WebSocket-Version` header does not equal `13`"))]
    SecWebSocketVersionHeaderNotEqual13 {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("`Sec-WebSocket-Key` header not present"))]
    SecWebSocketKeyHeaderNotPresent {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("request couldn't be upgraded to a WebSocket connection"))]
    ConnectionNotUpgradable {
        #[snafu(implicit)]
        location: Location,
    },
}

impl ResponseError for WebSocketError {
    fn as_status(&self) -> StatusCode {
        match self {
            WebSocketError::MethodNotGet { .. } => StatusCode::METHOD_NOT_ALLOWED,
            WebSocketError::ConnectionHeaderNotContainsUpgrade { .. }
            | WebSocketError::UpgradeHeaderNotEqualWebSocket { .. }
            | WebSocketError::SecWebSocketVersionHeaderNotEqual13 { .. }
            | WebSocketError::SecWebSocketKeyHeaderNotPresent { .. } => StatusCode::BAD_REQUEST,
            WebSocketError::ConnectionNotUpgradable { .. } => StatusCode::UPGRADE_REQUIRED,
        }
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::METHOD_NOT_ALLOWED);
        codes.insert(StatusCode::BAD_REQUEST);
        codes.insert(StatusCode::UPGRADE_REQUIRED);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        match self {
            WebSocketError::MethodNotGet { location }
            | WebSocketError::ConnectionHeaderNotContainsUpgrade { location }
            | WebSocketError::UpgradeHeaderNotEqualWebSocket { location }
            | WebSocketError::SecWebSocketVersionHeaderNotEqual13 { location }
            | WebSocketError::SecWebSocketKeyHeaderNotPresent { location }
            | WebSocketError::ConnectionNotUpgradable { location } => {
                stack.push(self, location);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventStreamErrorKind {
    Type,
    Id,
    Comment,
}

#[derive(Debug)]
pub struct EventStreamError {
    kind: EventStreamErrorKind,
    location: Location,
}

impl EventStreamError {
    #[track_caller]
    #[inline]
    fn new(kind: EventStreamErrorKind) -> Self {
        Self {
            kind,
            location: Location::caller(),
        }
    }

    #[track_caller]
    #[inline]
    pub fn invalid_type() -> Self {
        Self::new(EventStreamErrorKind::Type)
    }

    #[track_caller]
    #[inline]
    pub fn invalid_id() -> Self {
        Self::new(EventStreamErrorKind::Id)
    }

    #[track_caller]
    #[inline]
    pub fn invalid_comment() -> Self {
        Self::new(EventStreamErrorKind::Comment)
    }

    pub fn kind(&self) -> EventStreamErrorKind {
        self.kind
    }
}

impl fmt::Display for EventStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use EventStreamErrorKind::*;

        let field = match self.kind {
            Type => "event",
            Id => "id",
            Comment => "comment",
        };

        write!(
            f,
            "SSE `{}` field value cannot contain newlines or carriage returns",
            field
        )
    }
}

impl Error for EventStreamError {}

impl ResponseError for EventStreamError {
    fn as_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::INTERNAL_SERVER_ERROR);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        stack.push(self, &self.location);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_content_type() {
        let err = InvalidContentTypeSnafu {
            actual: "application/json",
            expected: ["text/plain", "text/html"],
        }
        .build();

        assert_eq!(
            err.to_string(),
            "invalid `content-type` header value: expected to be one of [\"text/plain\", \"text/html\"] but actually \"application/json\""
        );
    }
}
