use std::{collections::BTreeMap, future::Future};

use headers::{Connection, HeaderMapExt, SecWebsocketKey, SecWebsocketVersion, Upgrade};
use http::{
    header::{SEC_WEBSOCKET_PROTOCOL, UPGRADE},
    HeaderValue, Method, Version,
};
use hyper::{ext::Protocol, upgrade::OnUpgrade};
use predawn_core::{
    api_request::ApiRequestHead,
    from_request::FromRequestHead,
    openapi::{self, Schema},
    request::Head,
};
use snafu::OptionExt;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

use super::{WebSocket, WebSocketResponse};
use crate::response_error::{WebSocketError, *};

pub struct WebSocketRequest<F = DefaultOnFailedUpgrade> {
    pub config: WebSocketConfig,
    /// The chosen protocol sent in the `Sec-WebSocket-Protocol` header of the response.
    pub(crate) protocol: Option<HeaderValue>,
    /// `None` if HTTP/2+ WebSockets are used.
    pub(crate) sec_websocket_key: Option<SecWebsocketKey>,
    pub(crate) on_upgrade: OnUpgrade,
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
        let sec_websocket_key = if head.version <= Version::HTTP_11 {
            if head.method != Method::GET {
                return MethodNotGetSnafu.fail();
            }

            let connection_contains_upgrade = head
                .headers
                .typed_get::<Connection>()
                .is_some_and(|connection| connection.contains(UPGRADE));

            if !connection_contains_upgrade {
                return ConnectionHeaderNotContainsUpgradeSnafu.fail();
            }

            let upgrade_eq_websocket = head
                .headers
                .typed_get::<Upgrade>()
                .is_some_and(|upgrade| upgrade == Upgrade::websocket());

            if !upgrade_eq_websocket {
                return UpgradeHeaderNotEqualWebSocketSnafu.fail();
            }

            Some(
                head.headers
                    .typed_get::<SecWebsocketKey>()
                    .context(SecWebSocketKeyHeaderNotPresentSnafu)?
                    .clone(),
            )
        } else {
            if head.method != Method::CONNECT {
                return MethodNotConnectSnafu.fail();
            }

            if head
                .extensions
                .get::<Protocol>()
                .is_none_or(|p| p.as_str() != "websocket")
            {
                return ProtocolPseudoHeaderNotEqualWebSocketSnafu.fail();
            }

            None
        };

        let sec_websocket_version_eq_13 =
            head.headers.typed_get::<SecWebsocketVersion>() == Some(SecWebsocketVersion::V13);

        if !sec_websocket_version_eq_13 {
            return SecWebSocketVersionHeaderNotEqual13Snafu.fail();
        }

        let on_upgrade = head
            .extensions
            .remove::<OnUpgrade>()
            .context(ConnectionNotUpgradableSnafu)?;

        let sec_websocket_protocol = head.headers.get(SEC_WEBSOCKET_PROTOCOL).cloned();

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
        tracing::error!("failed to upgrade WebSocket: {:?}", error);
    }
}
