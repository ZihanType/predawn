use std::{collections::BTreeMap, convert::Infallible};

use bytes::{BufMut, Bytes, BytesMut};
use http::{
    header::{HeaderValue, CONTENT_TYPE},
    StatusCode,
};
use indexmap::IndexMap;
use mime::{APPLICATION, JSON};
use predawn_core::{
    api_request::ApiRequest,
    api_response::ApiResponse,
    body::RequestBody,
    from_request::FromRequest,
    impl_deref,
    into_response::IntoResponse,
    media_type::{
        has_media_type, MediaType, MultiRequestMediaType, MultiResponseMediaType, RequestMediaType,
        ResponseMediaType, SingleMediaType,
    },
    openapi::{self, Parameter, ReferenceOr, Schema},
    request::Head,
    response::{MultiResponse, Response, SingleResponse},
};
use predawn_schema::ToSchema;
use serde::{de::DeserializeOwned, Serialize};

use crate::response_error::{ReadJsonError, WriteJsonError};

#[derive(Debug, Default, Clone, Copy)]
pub struct Json<T>(pub T);

impl_deref!(Json);

impl<'a, T> FromRequest<'a> for Json<T>
where
    T: DeserializeOwned,
{
    type Error = ReadJsonError;

    async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        let content_type = head.content_type().unwrap_or_default();

        if <Self as RequestMediaType>::check_content_type(content_type) {
            let bytes = Bytes::from_request(head, body).await?;

            match crate::util::from_bytes(&bytes) {
                Ok(o) => Ok(Json(o)),
                Err(e) => Err(ReadJsonError::DeserializeJsonError(e)),
            }
        } else {
            Err(ReadJsonError::InvalidJsonContentType)
        }
    }
}

impl<T: ToSchema> ApiRequest for Json<T> {
    fn parameters(_: &mut IndexMap<String, ReferenceOr<Schema>>) -> Option<Vec<Parameter>> {
        None
    }

    fn request_body(
        schemas: &mut IndexMap<String, ReferenceOr<Schema>>,
    ) -> Option<openapi::RequestBody> {
        Some(openapi::RequestBody {
            content: <Self as MultiRequestMediaType>::content(schemas),
            required: true,
            ..Default::default()
        })
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
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
                    HeaderValue::from_static(<Self as MediaType>::MEDIA_TYPE),
                );

                Ok(response)
            }
            Err(err) => Err(WriteJsonError(err)),
        }
    }
}

impl<T: ToSchema> ApiResponse for Json<T> {
    fn responses(
        schemas: &mut IndexMap<String, ReferenceOr<Schema>>,
    ) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        Some(<Self as MultiResponse>::responses(schemas))
    }
}

impl<T> MediaType for Json<T> {
    const MEDIA_TYPE: &'static str = "application/json";
}

impl<T> RequestMediaType for Json<T> {
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

impl<T> ResponseMediaType for Json<T> {}

impl<T: ToSchema> SingleMediaType for Json<T> {
    fn media_type(schemas: &mut IndexMap<String, ReferenceOr<Schema>>) -> openapi::MediaType {
        openapi::MediaType {
            schema: Some(T::schema_ref(schemas)),
            ..Default::default()
        }
    }
}

impl<T: ToSchema> SingleResponse for Json<T> {
    fn response(schemas: &mut IndexMap<String, ReferenceOr<Schema>>) -> openapi::Response {
        openapi::Response {
            description: "JSON response".to_owned(),
            content: <Self as MultiResponseMediaType>::content(schemas),
            ..Default::default()
        }
    }
}
