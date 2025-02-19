use std::{collections::BTreeMap, convert::Infallible};

use bytes::Bytes;
use http::{
    header::{HeaderValue, CONTENT_TYPE},
    StatusCode,
};
use mime::{APPLICATION, JSON};
use predawn_core::{
    api_request::ApiRequest,
    api_response::ApiResponse,
    body::RequestBody,
    from_request::{FromRequest, OptionalFromRequest},
    impl_deref,
    into_response::IntoResponse,
    media_type::{
        has_media_type, MediaType, MultiRequestMediaType, MultiResponseMediaType, RequestMediaType,
        ResponseMediaType, SingleMediaType,
    },
    openapi::{self, Parameter, Schema},
    request::Head,
    response::{MultiResponse, Response, SingleResponse},
};
use predawn_schema::ToSchema;
use serde::{de::DeserializeOwned, Serialize};
use snafu::ResultExt;

use crate::response_error::{
    DeserializeJsonSnafu, InvalidJsonContentTypeSnafu, ReadJsonBytesSnafu, ReadJsonError,
    WriteJsonError, WriteJsonSnafu,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct Json<T>(pub T);

impl_deref!(Json);

impl<T> FromRequest for Json<T>
where
    T: DeserializeOwned,
{
    type Error = ReadJsonError;

    async fn from_request(head: &mut Head, body: RequestBody) -> Result<Self, Self::Error> {
        let content_type = head.content_type().unwrap_or_default();

        if !<Self as RequestMediaType>::check_content_type(content_type) {
            return InvalidJsonContentTypeSnafu.fail();
        }

        let bytes = Bytes::from_request(head, body)
            .await
            .context(ReadJsonBytesSnafu)?;

        let json = crate::util::deserialize_json(&bytes).context(DeserializeJsonSnafu)?;
        Ok(Json(json))
    }
}

impl<T> OptionalFromRequest for Json<T>
where
    T: DeserializeOwned,
{
    type Error = ReadJsonError;

    async fn from_request(head: &mut Head, body: RequestBody) -> Result<Option<Self>, Self::Error> {
        let Some(content_type) = head.content_type() else {
            return Ok(None);
        };

        if !<Self as RequestMediaType>::check_content_type(content_type) {
            return InvalidJsonContentTypeSnafu.fail();
        }

        let bytes = Bytes::from_request(head, body)
            .await
            .context(ReadJsonBytesSnafu)?;

        let json = crate::util::deserialize_json(&bytes).context(DeserializeJsonSnafu)?;
        Ok(Some(Json(json)))
    }
}

impl<T: ToSchema> ApiRequest for Json<T> {
    fn parameters(_: &mut BTreeMap<String, Schema>, _: &mut Vec<String>) -> Option<Vec<Parameter>> {
        None
    }

    fn request_body(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> Option<openapi::RequestBody> {
        Some(openapi::RequestBody {
            content: <Self as MultiRequestMediaType>::content(schemas, schemas_in_progress),
            required: <T as ToSchema>::REQUIRED,
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
        let mut response = crate::util::serialize_json(&self.0)
            .context(WriteJsonSnafu)?
            .into_response()
            .unwrap_or_else(|a: Infallible| match a {});

        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static(<Self as MediaType>::MEDIA_TYPE),
        );

        Ok(response)
    }
}

impl<T: ToSchema> ApiResponse for Json<T> {
    fn responses(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        Some(<Self as MultiResponse>::responses(
            schemas,
            schemas_in_progress,
        ))
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
    fn media_type(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> openapi::MediaType {
        openapi::MediaType {
            schema: Some(T::schema_ref(schemas, schemas_in_progress)),
            ..Default::default()
        }
    }
}

impl<T: ToSchema> SingleResponse for Json<T> {
    fn response(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> openapi::Response {
        openapi::Response {
            content: <Self as MultiResponseMediaType>::content(schemas, schemas_in_progress),
            ..Default::default()
        }
    }
}
