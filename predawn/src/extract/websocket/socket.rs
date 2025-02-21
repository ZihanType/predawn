use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::Stream;
use futures_util::{SinkExt, StreamExt, sink::Sink};
use http::HeaderValue;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{self, Message, protocol::CloseFrame},
};

#[derive(Debug)]
pub struct WebSocket {
    pub(crate) inner: WebSocketStream<TokioIo<Upgraded>>,
    pub(crate) protocol: Option<HeaderValue>,
}

impl WebSocket {
    /// Receive another message.
    ///
    /// Returns `None` if the stream has closed.
    #[inline(always)]
    pub async fn recv(&mut self) -> Option<Result<Message, tungstenite::Error>> {
        self.inner.next().await
    }

    /// Send a message.
    #[inline(always)]
    pub async fn send(&mut self, msg: Message) -> Result<(), tungstenite::Error> {
        self.inner.send(msg).await
    }

    /// Gracefully close this WebSocket.
    #[inline(always)]
    pub async fn close(&mut self, msg: Option<CloseFrame>) -> Result<(), tungstenite::Error> {
        self.inner.close(msg).await
    }

    /// Return the selected WebSocket subprotocol, if one has been chosen.
    #[inline(always)]
    pub fn protocol(&self) -> Option<&HeaderValue> {
        self.protocol.as_ref()
    }
}

impl Stream for WebSocket {
    type Item = Result<Message, tungstenite::Error>;

    #[inline(always)]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

impl Sink<Message> for WebSocket {
    type Error = tungstenite::Error;

    #[inline(always)]
    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_ready(cx)
    }

    #[inline(always)]
    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        Pin::new(&mut self.inner).start_send(item)
    }

    #[inline(always)]
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    #[inline(always)]
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_close(cx)
    }
}
