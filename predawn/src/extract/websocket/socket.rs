use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::Stream;
use futures_util::{sink::Sink, SinkExt, StreamExt};
use http::HeaderValue;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use tokio_tungstenite::{
    tungstenite::{self, Message},
    WebSocketStream,
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
    pub async fn recv(&mut self) -> Option<Result<Message, tungstenite::Error>> {
        self.inner.next().await
    }

    /// Send a message.
    pub async fn send(&mut self, msg: Message) -> Result<(), tungstenite::Error> {
        self.inner.send(msg).await
    }

    /// Gracefully close this WebSocket.
    pub async fn close(mut self) -> Result<(), tungstenite::Error> {
        self.inner.close(None).await
    }

    /// Return the selected WebSocket subprotocol, if one has been chosen.
    pub fn protocol(&self) -> Option<&HeaderValue> {
        self.protocol.as_ref()
    }
}

impl Stream for WebSocket {
    type Item = Result<Message, tungstenite::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

impl Sink<Message> for WebSocket {
    type Error = tungstenite::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_ready(cx)
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        Pin::new(&mut self.inner).start_send(item)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_close(cx)
    }
}
