use std::{collections::HashSet, fmt::Display, str::Utf8Error};

use async_trait::async_trait;
use http::StatusCode;
use predawn_core::{
    from_request::FromRequestHead,
    impl_deref,
    openapi::{Components, Parameter},
    request::Head,
    response_error::ResponseError,
};
use serde::Deserialize;

use crate::{path_params::PathParams, ToParameters};

mod de;

#[derive(Debug)]
pub struct Path<T>(pub T);

impl_deref!(Path);

#[async_trait]
impl<'a, T> FromRequestHead<'a> for Path<T>
where
    T: Deserialize<'a> + ToParameters,
{
    type Error = PathError;

    async fn from_request_head(head: &'a Head) -> Result<Self, Self::Error> {
        let params = match head.extensions.get::<PathParams>() {
            Some(PathParams::Params(params)) => params,
            Some(PathParams::InvalidUtf8InPathParam { key, error }) => {
                let err = PathError::InvalidUtf8InPathParam {
                    key: key.to_string(),
                    error: *error,
                };
                return Err(err);
            }
            None => {
                return Err(PathError::MissingPathParams);
            }
        };

        T::deserialize(de::PathDeserializer::new(params)).map(Path)
    }

    fn parameters(components: &mut Components) -> Option<Vec<Parameter>> {
        Some(
            <T as ToParameters>::parameters(components)
                .into_iter()
                .map(|parameter_data| Parameter::Path {
                    parameter_data,
                    style: Default::default(),
                })
                .collect(),
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("no paths parameters found for matched route")]
    MissingPathParams,

    /// The URI contained the wrong number of parameters.
    #[error("wrong number of parameters: expected {expected} but got {got}")]
    WrongNumberOfParameters {
        /// The number of actual parameters in the URI.
        got: usize,
        /// The number of expected parameters.
        expected: usize,
    },

    /// Failed to parse the value at a specific key into the expected type.
    ///
    /// This variant is used when deserializing into types that have named fields, such as structs.
    #[error("failed to parse `{key}` with value `{value:?}` to a `{expected_type}`")]
    ParseErrorAtKey {
        /// The key at which the value was located.
        key: String,
        /// The value from the URI.
        value: String,
        /// The expected type of the value.
        expected_type: &'static str,
    },

    /// A parameter contained text that, once percent decoded, wasn't valid UTF-8.
    #[error("{error} in `{key}`")]
    InvalidUtf8InPathParam {
        /// The key at which the invalid value was located.
        key: String,
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

impl serde::de::Error for PathError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Self::Message(msg.to_string())
    }
}
