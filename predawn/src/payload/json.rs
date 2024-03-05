use std::{
    collections::{BTreeMap, HashSet},
    convert::Infallible,
};

use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use http::{
    header::{HeaderValue, CONTENT_TYPE},
    StatusCode,
};
use mime::{APPLICATION, JSON};
use predawn_core::{
    body::RequestBody,
    from_request::{FromRequest, ReadBytesError},
    impl_deref,
    into_response::IntoResponse,
    media_type::{
        has_media_type, MultiRequestMediaType, MultiResponseMediaType, SingleMediaType,
        SingleRequestMediaType, SingleResponseMediaType,
    },
    openapi::{self, Components, MediaType, Parameter},
    request::Head,
    response::{MultiResponse, Response, SingleResponse},
    response_error::ResponseError,
};
use predawn_schema::ToSchema;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::error::Category;

#[derive(Debug, Default, Clone, Copy)]
pub struct Json<T>(pub T);

impl_deref!(Json);

#[async_trait]
impl<'a, T> FromRequest<'a> for Json<T>
where
    T: DeserializeOwned + ToSchema,
{
    type Error = ReadJsonError;

    async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        match head.content_type() {
            Some(content_type)
                if <Self as SingleRequestMediaType>::check_content_type(content_type) =>
            {
                let bytes = Bytes::from_request(head, body).await?;
                Self::from_bytes(&bytes)
            }
            _ => Err(ReadJsonError::InvalidJsonContentType),
        }
    }

    fn parameters(_: &mut Components) -> Option<Vec<Parameter>> {
        None
    }

    fn request_body(components: &mut Components) -> Option<openapi::RequestBody> {
        Some(openapi::RequestBody {
            description: Some("Extract JSON from request body".to_owned()),
            content: <Self as MultiRequestMediaType>::content(components),
            required: true,
            ..Default::default()
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReadJsonError {
    #[error("expected request with `{}: {}`", CONTENT_TYPE, <Json<bool> as SingleMediaType>::MEDIA_TYPE)]
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

impl<T> Json<T>
where
    T: DeserializeOwned,
{
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReadJsonError> {
        let deserializer = &mut serde_json::Deserializer::from_slice(bytes);

        match serde_path_to_error::deserialize(deserializer) {
            Ok(value) => Ok(Json(value)),
            Err(err) => {
                let error = match err.inner().classify() {
                    Category::Data => ReadJsonError::JsonDataError(err),
                    Category::Syntax | Category::Eof => ReadJsonError::JsonSyntaxError(err),
                    Category::Io => {
                        if cfg!(debug_assertions) {
                            // we don't use `serde_json::from_reader` and instead always buffer
                            // bodies first, so we shouldn't encounter any IO errors
                            unreachable!()
                        } else {
                            ReadJsonError::JsonSyntaxError(err)
                        }
                    }
                };
                Err(error)
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed to serialize response as JSON: {0}")]
pub struct WriteJsonError(#[from] serde_json::Error);

impl ResponseError for WriteJsonError {
    fn as_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::INTERNAL_SERVER_ERROR].into()
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize + ToSchema,
{
    type Error = WriteJsonError;

    fn into_response(self) -> Result<Response, Self::Error> {
        let mut buf = BytesMut::with_capacity(128).writer();

        match serde_json::to_writer(&mut buf, &self.0) {
            Ok(_) => {
                let mut response = buf
                    .into_inner()
                    .into_response()
                    .unwrap_or_else(|a: Infallible| match a {});

                response.headers_mut().insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static(<Self as SingleMediaType>::MEDIA_TYPE),
                );

                Ok(response)
            }
            Err(err) => Err(WriteJsonError(err)),
        }
    }

    fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        Some(<Self as MultiResponse>::responses(components))
    }
}

impl<T: ToSchema> SingleMediaType for Json<T> {
    const MEDIA_TYPE: &'static str = "application/json";

    fn media_type(components: &mut Components) -> MediaType {
        MediaType {
            schema: Some(T::schema_ref(components)),
            ..Default::default()
        }
    }
}

impl<T: ToSchema> SingleRequestMediaType for Json<T> {
    fn check_content_type(content_type: &str) -> bool {
        has_media_type(
            content_type,
            APPLICATION.as_str(),
            JSON.as_str(),
            JSON.as_str(),
            None,
        )
    }
}

impl<T: ToSchema> SingleResponseMediaType for Json<T> {}

impl<T: ToSchema> SingleResponse for Json<T> {
    fn response(components: &mut Components) -> openapi::Response {
        openapi::Response {
            description: "JSON response".to_owned(),
            content: <Self as MultiResponseMediaType>::content(components),
            ..Default::default()
        }
    }
}
