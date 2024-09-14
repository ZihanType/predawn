use std::{collections::BTreeMap, convert::Infallible, future::Future};

use headers::{Connection, HeaderMapExt, SecWebsocketAccept, Upgrade};
use http::{header::SEC_WEBSOCKET_PROTOCOL, StatusCode};
use hyper_util::rt::TokioIo;
use predawn_core::{
    api_response::ApiResponse,
    body::ResponseBody,
    into_response::IntoResponse,
    openapi::{self, Schema},
    response::{MultiResponse, Response, SingleResponse},
};
use tokio_tungstenite::{tungstenite::protocol::Role, WebSocketStream};

use super::{OnFailedUpgrade, WebSocket, WebSocketRequest};

#[derive(Debug)]
pub struct WebSocketResponse(Response);

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

        let mut response = http::Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .body(ResponseBody::empty())
            .unwrap();

        let headers = response.headers_mut();

        headers.typed_insert(Connection::upgrade());
        headers.typed_insert(Upgrade::websocket());
        headers.typed_insert(SecWebsocketAccept::from(sec_websocket_key));

        if let Some(protocol) = protocol {
            headers.insert(SEC_WEBSOCKET_PROTOCOL, protocol);
        }

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
