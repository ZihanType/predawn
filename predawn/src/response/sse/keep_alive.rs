use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use bytes::Bytes;
use pin_project_lite::pin_project;
use tokio::time::Sleep;

use super::event::Event;
use crate::response_error::EventStreamError;

#[derive(Debug, Clone)]
pub struct KeepAlive {
    comment: String,
    max_interval: Duration,
}

impl Default for KeepAlive {
    fn default() -> Self {
        Self {
            comment: String::from(": keep-alive\n\n"),
            max_interval: Duration::from_secs(15),
        }
    }
}

impl KeepAlive {
    pub fn comment<T: Into<String>>(self, comment: T) -> Self {
        fn inner(mut ka: KeepAlive, comment: String) -> KeepAlive {
            ka.comment = comment;
            ka
        }

        inner(self, comment.into())
    }

    pub fn interval(mut self, interval: Duration) -> Self {
        self.max_interval = interval;
        self
    }
}

pin_project! {
    #[derive(Debug)]
    pub(crate) struct KeepAliveStream {
        event: Bytes,
        max_interval: Duration,

        #[pin]
        alive_timer: Sleep,
    }
}

impl KeepAliveStream {
    pub(crate) fn new(keep_alive: KeepAlive) -> Result<Self, EventStreamError> {
        let KeepAlive {
            comment,
            max_interval,
        } = keep_alive;

        let event = Event::only_comment(comment);
        let event = event.as_bytes()?;

        Ok(Self {
            event,
            max_interval,
            alive_timer: tokio::time::sleep(max_interval),
        })
    }

    pub(crate) fn reset(self: Pin<&mut Self>) {
        let this = self.project();

        this.alive_timer
            .reset(tokio::time::Instant::now() + *this.max_interval);
    }

    pub(crate) fn poll_event(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Bytes> {
        let this = self.as_mut().project();

        std::task::ready!(this.alive_timer.poll(cx));

        let event = this.event.clone();

        self.reset();

        Poll::Ready(event)
    }
}
