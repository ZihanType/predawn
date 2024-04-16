use std::collections::BTreeMap;

use http::{
    header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    HeaderValue, StatusCode,
};
use predawn_core::{
    api_response::ApiResponse,
    into_response::IntoResponse,
    media_type::{MediaType, MultiResponseMediaType, ResponseMediaType, SingleMediaType},
    openapi::{self, Components},
    response::{MultiResponse, Response, SingleResponse},
};
use predawn_schema::ToSchema;

use crate::response_error::AttachmentError;

#[derive(Debug)]
pub struct Attachment<T> {
    data: T,
    file_name: Box<str>,
}

impl<T> Attachment<T> {
    pub fn new<S>(data: T, file_name: S) -> Self
    where
        S: Into<Box<str>>,
    {
        fn inner_new<T>(data: T, file_name: Box<str>) -> Attachment<T> {
            Attachment { data, file_name }
        }

        inner_new(data, file_name.into())
    }

    fn content_disposition<E>(file_name: Box<str>) -> Result<HeaderValue, AttachmentError<E>> {
        let content_disposition = format!("attachment; filename=\"{}\"", file_name);

        HeaderValue::from_str(&content_disposition)
            .map_err(|_| AttachmentError::InvalidContentDisposition { file_name })
    }
}

impl<T: IntoResponse + MediaType> IntoResponse for Attachment<T> {
    type Error = AttachmentError<T::Error>;

    fn into_response(self) -> Result<Response, Self::Error> {
        let Attachment { data, file_name } = self;

        let mut response = data.into_response().map_err(AttachmentError::Inner)?;

        let headers = response.headers_mut();

        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static(<Self as MediaType>::MEDIA_TYPE),
        );

        headers.insert(
            CONTENT_DISPOSITION,
            Self::content_disposition::<T::Error>(file_name)?,
        );

        Ok(response)
    }
}

impl<T: MediaType + ResponseMediaType> ApiResponse for Attachment<T> {
    fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        Some(<Self as MultiResponse>::responses(components))
    }
}

impl<T> ToSchema for Attachment<T> {
    fn schema() -> openapi::Schema {
        crate::util::binary_schema("Attachment")
    }
}

impl<T: MediaType> MediaType for Attachment<T> {
    const MEDIA_TYPE: &'static str = T::MEDIA_TYPE;
}

impl<T: ResponseMediaType> ResponseMediaType for Attachment<T> {}

impl<T> SingleMediaType for Attachment<T> {
    fn media_type(components: &mut Components) -> openapi::MediaType {
        openapi::MediaType {
            schema: Some(<Self as ToSchema>::schema_ref(components)),
            ..Default::default()
        }
    }
}

impl<T: MediaType + ResponseMediaType> SingleResponse for Attachment<T> {
    fn response(components: &mut Components) -> openapi::Response {
        openapi::Response {
            content: <Self as MultiResponseMediaType>::content(components),
            ..Default::default()
        }
    }
}
