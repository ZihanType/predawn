use std::{collections::BTreeMap, convert::Infallible};

use bytes::Bytes;
use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use mime::{APPLICATION, WWW_FORM_URLENCODED};
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
    DeserializeFormSnafu, InvalidFormContentTypeSnafu, ReadFormBytesSnafu, ReadFormError,
    WriteFormError, WriteFormSnafu,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct Form<T>(pub T);

impl_deref!(Form);

impl<T> FromRequest for Form<T>
where
    T: DeserializeOwned,
{
    type Error = ReadFormError;

    async fn from_request(head: &mut Head, body: RequestBody) -> Result<Self, Self::Error> {
        let content_type = head.content_type().unwrap_or_default();

        if !<Self as RequestMediaType>::check_content_type(content_type) {
            return InvalidFormContentTypeSnafu.fail();
        }

        let bytes = Bytes::from_request(head, body)
            .await
            .context(ReadFormBytesSnafu)?;

        let form = crate::util::deserialize_form(&bytes).context(DeserializeFormSnafu)?;
        Ok(Form(form))
    }
}

impl<T> OptionalFromRequest for Form<T>
where
    T: DeserializeOwned,
{
    type Error = ReadFormError;

    async fn from_request(head: &mut Head, body: RequestBody) -> Result<Option<Self>, Self::Error> {
        let Some(content_type) = head.content_type() else {
            return Ok(None);
        };

        if !<Self as RequestMediaType>::check_content_type(content_type) {
            return InvalidFormContentTypeSnafu.fail();
        }

        let bytes = Bytes::from_request(head, body)
            .await
            .context(ReadFormBytesSnafu)?;

        let form = crate::util::deserialize_form(&bytes).context(DeserializeFormSnafu)?;
        Ok(Some(Form(form)))
    }
}

impl<T: ToSchema> ApiRequest for Form<T> {
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

impl<T> IntoResponse for Form<T>
where
    T: Serialize,
{
    type Error = WriteFormError;

    fn into_response(self) -> Result<Response, Self::Error> {
        let mut response = crate::util::serialize_form(&self.0)
            .context(WriteFormSnafu)?
            .into_response()
            .unwrap_or_else(|a: Infallible| match a {});

        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static(<Self as MediaType>::MEDIA_TYPE),
        );

        Ok(response)
    }
}

impl<T: ToSchema> ApiResponse for Form<T> {
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

impl<T> MediaType for Form<T> {
    const MEDIA_TYPE: &'static str = "application/x-www-form-urlencoded";
}

impl<T> RequestMediaType for Form<T> {
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

impl<T> ResponseMediaType for Form<T> {}

impl<T: ToSchema> SingleMediaType for Form<T> {
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

impl<T: ToSchema> SingleResponse for Form<T> {
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
