use std::{collections::BTreeMap, future::Future};

use headers::{Connection, HeaderMapExt, SecWebsocketKey, SecWebsocketVersion, Upgrade};
use http::{header, HeaderValue, Method};
use hyper::upgrade::OnUpgrade;
use predawn_core::{
    api_request::ApiRequestHead,
    from_request::FromRequestHead,
    openapi::{self, Schema},
    request::Head,
};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

use super::{WebSocket, WebSocketResponse};
use crate::response_error::WebSocketError;

pub struct WebSocketRequest<F = DefaultOnFailedUpgrade> {
    pub config: WebSocketConfig,
    /// The chosen protocol sent in the `Sec-WebSocket-Protocol` header of the response.
    pub(crate) protocol: Option<HeaderValue>,
    pub(crate) sec_websocket_key: SecWebsocketKey,
    pub(crate) on_upgrade: hyper::upgrade::OnUpgrade,
    pub(crate) on_failed_upgrade: F,
    pub(crate) sec_websocket_protocol: Option<HeaderValue>,
}

impl<F> std::fmt::Debug for WebSocketRequest<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebSocketRequest")
            .field("config", &self.config)
            .field("protocol", &self.protocol)
            .field("sec_websocket_key", &self.sec_websocket_key)
            .field("sec_websocket_protocol", &self.sec_websocket_protocol)
            .finish_non_exhaustive()
    }
}

impl<F> WebSocketRequest<F> {
    pub fn protocols<I>(mut self, protocols: I) -> (Self, Option<I::Item>)
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut invalid_header_value = None;

        if let Some(protocols_in_request) = self
            .sec_websocket_protocol
            .as_ref()
            .and_then(|p| p.to_str().ok())
        {
            self.protocol = protocols
                .into_iter()
                .find(|protocol| {
                    protocols_in_request
                        .split(',')
                        .any(|protocol_in_request| protocol_in_request.trim() == protocol.as_ref())
                })
                .and_then(|protocol| match HeaderValue::from_str(protocol.as_ref()) {
                    Ok(protocol) => Some(protocol),
                    Err(_) => {
                        invalid_header_value = Some(protocol);
                        None
                    }
                });
        }

        (self, invalid_header_value)
    }

    pub fn on_failed_upgrade<C>(self, callback: C) -> WebSocketRequest<C>
    where
        C: OnFailedUpgrade,
    {
        WebSocketRequest {
            config: self.config,
            protocol: self.protocol,
            sec_websocket_key: self.sec_websocket_key,
            on_upgrade: self.on_upgrade,
            on_failed_upgrade: callback,
            sec_websocket_protocol: self.sec_websocket_protocol,
        }
    }

    pub fn on_upgrade<C, Fut>(self, callback: C) -> WebSocketResponse
    where
        F: OnFailedUpgrade,
        C: FnOnce(WebSocket) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        WebSocketResponse::new(self, callback)
    }
}

impl ApiRequestHead for WebSocketRequest {
    fn parameters(
        _: &mut BTreeMap<String, Schema>,
        _: &mut Vec<String>,
    ) -> Option<Vec<openapi::Parameter>> {
        None
    }
}

impl<'a> FromRequestHead<'a> for WebSocketRequest {
    type Error = WebSocketError;

    async fn from_request_head(head: &'a mut Head) -> Result<Self, Self::Error> {
        if head.method != Method::GET {
            return Err(WebSocketError::MethodNotGet);
        }

        let connection_contains_upgrade = head
            .headers
            .typed_get::<Connection>()
            .map_or(false, |connection| connection.contains(header::UPGRADE));

        if !connection_contains_upgrade {
            return Err(WebSocketError::ConnectionHeaderNotContainsUpgrade);
        }

        let upgrade_eq_websocket = head
            .headers
            .typed_get::<Upgrade>()
            .map_or(false, |upgrade| upgrade == Upgrade::websocket());

        if !upgrade_eq_websocket {
            return Err(WebSocketError::UpgradeHeaderNotEqualWebSocket);
        }

        let sec_websocket_version_eq_13 = head
            .headers
            .typed_get::<SecWebsocketVersion>()
            .map_or(false, |version| version == SecWebsocketVersion::V13);

        if !sec_websocket_version_eq_13 {
            return Err(WebSocketError::SecWebSocketVersionHeaderNotEqual13);
        }

        let sec_websocket_key = head
            .headers
            .typed_get::<SecWebsocketKey>()
            .ok_or(WebSocketError::SecWebSocketKeyHeaderNotPresent)?
            .clone();

        let on_upgrade = head
            .extensions
            .remove::<OnUpgrade>()
            .ok_or(WebSocketError::ConnectionNotUpgradable)?;

        let sec_websocket_protocol = head.headers.get(header::SEC_WEBSOCKET_PROTOCOL).cloned();

        Ok(Self {
            config: Default::default(),
            protocol: None,
            sec_websocket_key,
            on_upgrade,
            sec_websocket_protocol,
            on_failed_upgrade: DefaultOnFailedUpgrade,
        })
    }
}

pub trait OnFailedUpgrade: Send + 'static {
    fn call(self, error: hyper::Error);
}

impl<F> OnFailedUpgrade for F
where
    F: FnOnce(hyper::Error) + Send + 'static,
{
    fn call(self, error: hyper::Error) {
        self(error)
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct DefaultOnFailedUpgrade;

impl OnFailedUpgrade for DefaultOnFailedUpgrade {
    fn call(self, error: hyper::Error) {
        tracing::error!("WebSocket upgrade failed: {:?}", error);
    }
}
