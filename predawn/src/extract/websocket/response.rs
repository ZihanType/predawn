use std::{collections::BTreeMap, convert::Infallible, future::Future};

use http::{header, StatusCode};
use hyper_util::rt::TokioIo;
use predawn_core::{
    api_response::ApiResponse,
    body::ResponseBody,
    into_response::IntoResponse,
    openapi::{self, Schema},
    response::{MultiResponse, Response, SingleResponse},
};
use tokio_tungstenite::{
    tungstenite::{handshake::derive_accept_key, protocol::Role},
    WebSocketStream,
};

use super::{OnFailedUpgrade, WebSocket, WebSocketRequest};

#[derive(Debug)]
pub struct WebSocketResponse(pub(crate) Response);

impl WebSocketResponse {
    pub(crate) fn new<F, C, Fut>(request: WebSocketRequest<F>, callback: C) -> WebSocketResponse
    where
        F: OnFailedUpgrade,
        C: FnOnce(WebSocket) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let WebSocketRequest {
            config,
            protocol,
            sec_websocket_key,
            on_upgrade,
            on_failed_upgrade,
            sec_websocket_protocol: _,
        } = request;

        {
            let protocol = protocol.clone();

            tokio::spawn(async move {
                let upgraded = match on_upgrade.await {
                    Ok(upgraded) => upgraded,
                    Err(err) => {
                        on_failed_upgrade.call(err);
                        return;
                    }
                };
                let upgraded = TokioIo::new(upgraded);

                let socket =
                    WebSocketStream::from_raw_socket(upgraded, Role::Server, Some(config)).await;
                let socket = WebSocket {
                    inner: socket,
                    protocol,
                };

                callback(socket).await;
            });
        }

        let mut builder = http::Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header(header::CONNECTION, "upgrade")
            .header(header::UPGRADE, "websocket")
            .header(
                header::SEC_WEBSOCKET_ACCEPT,
                derive_accept_key(sec_websocket_key.as_bytes()),
            );

        if let Some(protocol) = protocol {
            builder = builder.header(header::SEC_WEBSOCKET_PROTOCOL, protocol);
        }

        let response = builder.body(ResponseBody::empty()).unwrap();
        WebSocketResponse(response)
    }
}

impl IntoResponse for WebSocketResponse {
    type Error = Infallible;

    fn into_response(self) -> Result<Response, Self::Error> {
        Ok(self.0)
    }
}

impl SingleResponse for WebSocketResponse {
    const STATUS_CODE: u16 = 101;

    fn response(_: &mut BTreeMap<String, Schema>, _: &mut Vec<String>) -> openapi::Response {
        openapi::Response {
            description: "A WebSocket response".to_string(),
            ..Default::default()
        }
    }
}

impl ApiResponse for WebSocketResponse {
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
