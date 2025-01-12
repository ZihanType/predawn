use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_core::{Stream, TryStream};
use futures_util::TryStreamExt;
use http::header::{CACHE_CONTROL, CONTENT_TYPE};
use pin_project_lite::pin_project;
use predawn_core::{
    body::ResponseBody, error::BoxError, media_type::MediaType, response::Response,
};
use serde::Serialize;

use super::{keep_alive::KeepAliveStream, Event, EventStream, KeepAlive};
use crate::response_error::EventStreamError;

pub struct EventStreamBuilder<F> {
    pub(crate) keep_alive: Option<KeepAlive>,
    pub(crate) _marker: PhantomData<F>,
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
        C: OnCreateEvent<Data = F::Data>,
    {
        EventStreamBuilder {
            keep_alive: self.keep_alive,
            _marker: PhantomData,
        }
    }

    pub fn build<S>(self, stream: S) -> EventStream<F::Data>
    where
        S: TryStream + Send + 'static,
        S::Ok: Into<F::Item> + Send,
        S::Error: Into<BoxError>,
    {
        EventStream {
            result: inner_build(self, stream),
            _marker: PhantomData,
        }
    }
}

fn inner_build<F, S>(
    builder: EventStreamBuilder<F>,
    stream: S,
) -> Result<Response, EventStreamError>
where
    F: OnCreateEvent,
    S: TryStream + Send + 'static,
    S::Ok: Into<F::Item> + Send,
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
        stream: stream.map_err(Into::into).and_then(|item| async move {
            let item = item.into();

            let data = F::data(&item);

            let event = Event::data(data).map_err(Box::new)?;
            let event = F::modify_event(item, event).map_err(Box::new)?;

            Ok::<_, BoxError>(event.as_bytes())
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

    fn modify_event(item: Self::Item, event: Event) -> Result<Event, EventStreamError>;
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

    fn modify_event(_: Self::Item, event: Event) -> Result<Event, EventStreamError> {
        Ok(event)
    }
}
