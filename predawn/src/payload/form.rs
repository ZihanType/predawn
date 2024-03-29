use std::{
    collections::{BTreeMap, HashSet},
    convert::Infallible,
};

use async_trait::async_trait;
use bytes::Bytes;
use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use mime::{APPLICATION, WWW_FORM_URLENCODED};
use predawn_core::{
    body::RequestBody,
    from_request::{FromRequest, ReadBytesError},
    impl_deref,
    into_response::IntoResponse,
    media_type::{
        has_media_type, MultiRequestMediaType, MultiResponseMediaType, SingleMediaType,
        SingleRequestMediaType, SingleResponseMediaType,
    },
    openapi::{self, Components, Parameter},
    request::Head,
    response::{MultiResponse, Response, SingleResponse},
    response_error::ResponseError,
};
use predawn_schema::ToSchema;
use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug, Default, Clone, Copy)]
pub struct Form<T>(pub T);

impl_deref!(Form);

#[derive(Debug, thiserror::Error)]
pub enum ReadFormError {
    #[error("{0}")]
    ReadBytesError(#[from] ReadBytesError),
    #[error("expected request with `{}: {}`", CONTENT_TYPE, <Form<bool> as SingleMediaType>::MEDIA_TYPE)]
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

#[async_trait]
impl<'a, T> FromRequest<'a> for Form<T>
where
    T: DeserializeOwned + ToSchema,
{
    type Error = ReadFormError;

    async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        let content_type = head.content_type().unwrap_or_default();

        if <Self as SingleRequestMediaType>::check_content_type(content_type) {
            let bytes = Bytes::from_request(head, body).await?;

            match serde_html_form::from_bytes::<T>(&bytes) {
                Ok(value) => Ok(Form(value)),
                Err(err) => Err(ReadFormError::FormDeserializeError(err)),
            }
        } else {
            Err(ReadFormError::InvalidFormContentType)
        }
    }

    fn parameters(_: &mut Components) -> Option<Vec<Parameter>> {
        None
    }

    fn request_body(components: &mut Components) -> Option<openapi::RequestBody> {
        Some(openapi::RequestBody {
            description: Some("Extract FORM from request body".to_owned()),
            content: <Self as MultiRequestMediaType>::content(components),
            required: true,
            ..Default::default()
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed to serialize form data: {0}")]
pub struct WriteFormError(#[from] serde_html_form::ser::Error);

impl ResponseError for WriteFormError {
    fn as_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::INTERNAL_SERVER_ERROR].into()
    }
}

impl<T> IntoResponse for Form<T>
where
    T: Serialize + ToSchema,
{
    type Error = WriteFormError;

    fn into_response(self) -> Result<Response, Self::Error> {
        let mut response = serde_html_form::to_string(&self.0)
            .map_err(WriteFormError)?
            .into_response()
            .unwrap_or_else(|a: Infallible| match a {});

        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static(<Self as SingleMediaType>::MEDIA_TYPE),
        );

        Ok(response)
    }

    fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        Some(<Self as MultiResponse>::responses(components))
    }
}

impl<T: ToSchema> SingleMediaType for Form<T> {
    const MEDIA_TYPE: &'static str = "application/x-www-form-urlencoded";

    fn media_type(components: &mut Components) -> openapi::MediaType {
        openapi::MediaType {
            schema: Some(T::schema_ref(components)),
            ..Default::default()
        }
    }
}

impl<T: ToSchema> SingleRequestMediaType for Form<T> {
    fn check_content_type(content_type: &str) -> bool {
        has_media_type(
            content_type,
            APPLICATION.as_str(),
            WWW_FORM_URLENCODED.as_str(),
            WWW_FORM_URLENCODED.as_str(),
            None,
        )
    }
}

impl<T: ToSchema> SingleResponseMediaType for Form<T> {}

impl<T: ToSchema> SingleResponse for Form<T> {
    fn response(components: &mut Components) -> openapi::Response {
        openapi::Response {
            description: "FORM response".to_owned(),
            content: <Self as MultiResponseMediaType>::content(components),
            ..Default::default()
        }
    }
}
