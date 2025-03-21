use std::collections::BTreeMap;

use headers::{Connection, Header, HeaderMapExt, SecWebsocketKey, SecWebsocketVersion, Upgrade};
use http::{
    HeaderValue, Method, Version,
    header::{SEC_WEBSOCKET_PROTOCOL, UPGRADE},
};
use hyper::{ext::Protocol, upgrade::OnUpgrade};
use predawn_core::{
    api_request::ApiRequestHead,
    from_request::FromRequestHead,
    openapi::{self, ParameterData, ParameterSchemaOrContent, Schema},
    request::Head,
};
use predawn_schema::ToSchema;
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
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> Option<Vec<openapi::Parameter>> {
        let str_schema = <String as ToSchema>::schema_ref(schemas, schemas_in_progress);
        let u8_schema = <u8 as ToSchema>::schema_ref(schemas, schemas_in_progress);

        let connection = openapi::Parameter::Header {
            parameter_data: ParameterData {
                name: "connection".to_string(),
                description: Default::default(),
                required: false, // it is not required when the request is HTTP/2+
                deprecated: Default::default(),
                format: ParameterSchemaOrContent::Schema(str_schema.clone()),
                example: Some(serde_json::json!("upgrade")),
                examples: Default::default(),
                explode: Default::default(),
                extensions: Default::default(),
            },
            style: Default::default(),
        };

        let upgrade = openapi::Parameter::Header {
            parameter_data: ParameterData {
                name: "upgrade".to_string(),
                description: Default::default(),
                required: false, // it is not required when the request is HTTP/2+
                deprecated: Default::default(),
                format: ParameterSchemaOrContent::Schema(str_schema.clone()),
                example: Some(serde_json::json!("websocket")),
                examples: Default::default(),
                explode: Default::default(),
                extensions: Default::default(),
            },
            style: Default::default(),
        };

        let sec_websocket_key = openapi::Parameter::Header {
            parameter_data: ParameterData {
                name: "sec-websocket-key".to_string(),
                description: Default::default(),
                required: false, // it is not required when the request is HTTP/2+
                deprecated: Default::default(),
                format: ParameterSchemaOrContent::Schema(str_schema),
                example: Some(serde_json::json!("CgpkGOwMZTwleAoYngEPpQ==")),
                examples: Default::default(),
                explode: Default::default(),
                extensions: Default::default(),
            },
            style: Default::default(),
        };

        let sec_websocket_version = openapi::Parameter::Header {
            parameter_data: ParameterData {
                name: "sec-websocket-version".to_string(),
                description: Default::default(),
                required: true, // it is required in both HTTP/1.1 and HTTP/2+
                deprecated: Default::default(),
                format: ParameterSchemaOrContent::Schema(u8_schema),
                example: Some(serde_json::json!(13)),
                examples: Default::default(),
                explode: Default::default(),
                extensions: Default::default(),
            },
            style: Default::default(),
        };

        Some(vec![
            connection,
            upgrade,
            sec_websocket_key,
            sec_websocket_version,
        ])
    }
}

impl FromRequestHead for WebSocketRequest {
    type Error = WebSocketError;

    async fn from_request_head(head: &mut Head) -> Result<Self, Self::Error> {
        const WEBSOCKET: &str = "websocket";

        let mut buf = Vec::<HeaderValue>::with_capacity(1);

        macro_rules! extract_value {
            ($header:expr) => {{
                Header::encode(&$header, &mut buf);
                debug_assert_eq!(buf.len(), 1);
                buf.pop().unwrap()
            }};
        }

        let sec_websocket_key = if head.version <= Version::HTTP_11 {
            if head.method != Method::GET {
                return MethodNotGetSnafu.fail();
            }

            let Some(connection) = head.headers.typed_get::<Connection>() else {
                return MissingConnectionHeaderSnafu.fail();
            };

            if !connection.contains(UPGRADE) {
                let value = extract_value!(connection);
                let value = value.to_str().ok().map(Box::from);

                return ConnectionHeaderNotContainsUpgradeSnafu { value }.fail();
            }

            let Some(upgrade) = head.headers.typed_get::<Upgrade>() else {
                return MissingUpgradeHeaderSnafu.fail();
            };

            let upgrade = extract_value!(upgrade);

            if !upgrade
                .as_bytes()
                .eq_ignore_ascii_case(WEBSOCKET.as_bytes())
            {
                let value = upgrade.to_str().ok().map(Box::from);

                return UpgradeHeaderNotEqualWebSocketSnafu { value }.fail();
            }

            Some(
                head.headers
                    .typed_get::<SecWebsocketKey>()
                    .context(MissingSecWebSocketKeyHeaderSnafu)?
                    .clone(),
            )
        } else {
            if head.method != Method::CONNECT {
                return MethodNotConnectSnafu.fail();
            }

            let Some(protocol) = head.extensions.get::<Protocol>() else {
                return MissingProtocolPseudoHeaderSnafu.fail();
            };

            let protocol = protocol.as_str();

            if protocol != WEBSOCKET {
                return ProtocolPseudoHeaderNotEqualWebSocketSnafu {
                    value: Box::from(protocol),
                }
                .fail();
            }

            None
        };

        let Some(sec_websocket_version) = head.headers.typed_get::<SecWebsocketVersion>() else {
            return MissingSecWebSocketVersionHeaderSnafu.fail();
        };

        if sec_websocket_version != SecWebsocketVersion::V13 {
            let value = extract_value!(sec_websocket_version);
            let value = value.to_str().ok().map(Box::from);

            return SecWebSocketVersionHeaderNotEqual13Snafu { value }.fail();
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
