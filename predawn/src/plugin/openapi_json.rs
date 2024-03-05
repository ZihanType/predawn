use std::{collections::HashMap, sync::Arc};

use http::{header::CONTENT_TYPE, HeaderValue, Method};
use mime::APPLICATION_JSON;
use predawn_core::{openapi::OpenAPI, response::Response};
use rudi::{Context, Singleton};

use super::Plugin;
use crate::{
    config::openapi::OpenAPIConfig,
    handler::{handler_fn, Handler},
    normalized_path::NormalizedPath,
};

#[derive(Clone, Copy)]
pub struct OpenAPIJson;

impl Plugin for OpenAPIJson {
    #[track_caller]
    fn create_route(
        self: Arc<Self>,
        cx: &mut Context,
    ) -> (NormalizedPath, HashMap<Method, Arc<dyn Handler>>) {
        let json_path = cx.resolve::<OpenAPIConfig>().normalized_json_path();

        let json = serde_json::to_string_pretty(&cx.get_single::<OpenAPI>())
            .expect("`OpenAPI` serialize failed");

        let mut map = HashMap::with_capacity(1);

        let handler = handler_fn(move |_| {
            let json = json.clone();

            async move {
                let mut response = Response::new(json.into());
                response.headers_mut().insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static(APPLICATION_JSON.as_ref()),
                );
                Ok(response)
            }
        });

        let handler: Arc<dyn Handler> = Arc::new(handler);

        map.insert(Method::GET, handler);

        (json_path, map)
    }
}

#[Singleton]
fn OpenAPIJsonRegister() -> OpenAPIJson {
    OpenAPIJson
}

#[Singleton(name = std::any::type_name::<OpenAPIJson>())]
fn OpenAPIJsonToPlugin(json: OpenAPIJson) -> Arc<dyn Plugin> {
    Arc::new(json)
}
