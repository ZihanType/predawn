use std::{
    collections::BTreeMap,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_core::{Stream, TryStream};
use futures_util::TryStreamExt;
use http::{
    header::{CACHE_CONTROL, CONTENT_TYPE},
    StatusCode,
};
use pin_project_lite::pin_project;
use predawn_core::{
    api_response::ApiResponse,
    body::ResponseBody,
    error::BoxError,
    into_response::IntoResponse,
    media_type::{MediaType, MultiResponseMediaType, ResponseMediaType, SingleMediaType},
    openapi::{self, AnySchema, ReferenceOr, Schema, SchemaKind},
    response::{MultiResponse, Response, SingleResponse},
};
use predawn_schema::ToSchema;
use serde::Serialize;

use super::{event::Event, keep_alive::KeepAlive};
use crate::{response::sse::keep_alive::KeepAliveStream, response_error::EventStreamError};

pub struct EventStream<T> {
    result: Result<Response, EventStreamError>,
    _marker: PhantomData<T>,
}

impl<T> EventStream<T> {
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

pub struct EventStreamBuilder<F> {
    keep_alive: Option<KeepAlive>,
    _marker: PhantomData<F>,
}

impl<F> EventStreamBuilder<F> {
    pub fn keep_alive(mut self, keep_alive: KeepAlive) -> Self {
        self.keep_alive = Some(keep_alive);
        self
    }
}

impl<F: OnCreateEvent> EventStreamBuilder<F> {
    pub fn on_create_event<C>(self) -> EventStreamBuilder<C>
    where
        C: OnCreateEvent<Item = F::Item>,
    {
        EventStreamBuilder {
            keep_alive: self.keep_alive,
            _marker: PhantomData,
        }
    }

    pub fn build<S>(self, stream: S) -> EventStream<F::Data>
    where
        S: TryStream<Ok = F::Item> + Send + 'static,
        S::Error: Into<BoxError>,
    {
        EventStream {
            result: inner_build(self, stream),
            _marker: PhantomData,
        }
    }
}

fn inner_build<C, S>(
    builder: EventStreamBuilder<C>,
    stream: S,
) -> Result<Response, EventStreamError>
where
    C: OnCreateEvent,
    S: TryStream<Ok = C::Item> + Send + 'static,
    S::Error: Into<BoxError>,
{
    pin_project! {
        struct SseStream<S> {
            #[pin]
            stream: S,
            #[pin]
            keep_alive: Option<KeepAliveStream>,
        }
    }

    impl<S> Stream for SseStream<S>
    where
        S: Stream<Item = Result<Bytes, BoxError>> + Send + 'static,
    {
        type Item = S::Item;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let mut this = self.project();

            match this.stream.try_poll_next_unpin(cx) {
                Poll::Pending => {
                    if let Some(keep_alive) = this.keep_alive.as_pin_mut() {
                        keep_alive.poll_event(cx).map(|e| Some(Ok(e)))
                    } else {
                        Poll::Pending
                    }
                }
                ok @ Poll::Ready(Some(Ok(_))) => {
                    if let Some(keep_alive) = this.keep_alive.as_pin_mut() {
                        keep_alive.reset();
                    }

                    ok
                }
                other => other,
            }
        }
    }

    let stream = SseStream {
        stream: stream.map_err(Into::into).and_then(|t| async move {
            let data = C::data(&t);
            let data = serde_json::to_string(data).map_err(Box::new)?;

            let mut evt = Event::data(data);
            C::modify_event(&t, &mut evt);

            let bytes = evt.as_bytes().map_err(Box::new)?;

            Ok::<_, BoxError>(bytes)
        }),
        keep_alive: builder.keep_alive.map(KeepAliveStream::new).transpose()?,
    };

    let body = ResponseBody::from_stream(stream);

    let response = http::Response::builder()
        .header(CONTENT_TYPE, EventStream::<()>::MEDIA_TYPE)
        .header(CACHE_CONTROL, "no-cache")
        .header("X-Accel-Buffering", "no")
        .body(body)
        .unwrap();

    Ok(response)
}

pub trait OnCreateEvent {
    type Item: Send + 'static;
    type Data: Serialize;

    fn data(item: &Self::Item) -> &Self::Data;

    fn modify_event(item: &Self::Item, event: &mut Event);
}

#[derive(Debug)]
pub struct DefaultOnCreateEvent<T> {
    _marker: PhantomData<T>,
}

impl<T> OnCreateEvent for DefaultOnCreateEvent<T>
where
    T: Serialize + Send + 'static,
{
    type Data = T;
    type Item = T;

    fn data(item: &Self::Item) -> &Self::Data {
        item
    }

    fn modify_event(_: &Self::Item, _: &mut Event) {}
}
