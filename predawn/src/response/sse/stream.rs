use std::{collections::BTreeMap, marker::PhantomData};

use futures_core::TryStream;
use http::StatusCode;
use predawn_core::{
    api_response::ApiResponse,
    error::BoxError,
    into_response::IntoResponse,
    media_type::{MediaType, MultiResponseMediaType, ResponseMediaType, SingleMediaType},
    openapi::{self, AnySchema, ReferenceOr, Schema, SchemaKind},
    response::{MultiResponse, Response, SingleResponse},
};
use predawn_schema::ToSchema;
use serde::Serialize;

use super::{DefaultOnCreateEvent, EventStreamBuilder};
use crate::response_error::EventStreamError;

pub struct EventStream<T> {
    pub(crate) result: Result<Response, EventStreamError>,
    pub(crate) _marker: PhantomData<T>,
}

impl<T> EventStream<T> {
    pub fn new<S>(stream: S) -> Self
    where
        T: Serialize + Send + 'static,
        S: TryStream + Send + 'static,
        S::Ok: Into<T> + Send,
        S::Error: Into<BoxError>,
    {
        Self::builder().build(stream)
    }

    pub fn builder() -> EventStreamBuilder<DefaultOnCreateEvent<T>> {
        EventStreamBuilder {
            keep_alive: None,
            _marker: PhantomData,
        }
    }
}

impl<T> IntoResponse for EventStream<T> {
    type Error = EventStreamError;

    fn into_response(self) -> Result<Response, Self::Error> {
        self.result
    }
}

impl<T> MediaType for EventStream<T> {
    const MEDIA_TYPE: &'static str = "text/event-stream";
}

impl<T> ResponseMediaType for EventStream<T> {}

impl<T: ToSchema> SingleMediaType for EventStream<T> {
    fn media_type(
        schemas: &mut BTreeMap<String, openapi::Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> openapi::MediaType {
        let schema = Schema {
            schema_data: Default::default(),
            schema_kind: SchemaKind::Any(AnySchema {
                typ: Some("array".into()),
                items: Some(T::schema_ref_box(schemas, schemas_in_progress)),
                format: Some("event-stream".into()),
                ..Default::default()
            }),
        };

        openapi::MediaType {
            schema: Some(ReferenceOr::Item(schema)),
            ..Default::default()
        }
    }
}

impl<T: ToSchema> SingleResponse for EventStream<T> {
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

impl<T: ToSchema> ApiResponse for EventStream<T> {
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
