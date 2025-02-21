use std::sync::Arc;

use http::Method;
use indexmap::IndexMap;
use predawn_core::openapi::OpenAPI;
use rudi::{Context, Singleton};

use super::Plugin;
use crate::{
    config::openapi::OpenAPIConfig,
    handler::{DynHandler, handler_fn},
    normalized_path::NormalizedPath,
    payload::Json,
};

#[derive(Clone, Copy)]
pub struct OpenAPIJson;

impl Plugin for OpenAPIJson {
    fn create_route(
        self: Arc<Self>,
        cx: &mut Context,
    ) -> (NormalizedPath, IndexMap<Method, DynHandler>) {
        let json_path = cx.resolve::<OpenAPIConfig>().json_path;

        let api = cx.resolve::<OpenAPI>();

        let handler = handler_fn(move |_| {
            let api = api.clone();
            async move { Ok(Json(api)) }
        });
        let handler = DynHandler::new(handler);

        let mut map = IndexMap::with_capacity(1);
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
