mod openapi_explorer;
mod rapidoc;
mod redoc;
mod scalar;
mod swagger_ui;

use http::{header::CONTENT_TYPE, HeaderValue, Method};
use indexmap::IndexMap;
use mime::TEXT_HTML_UTF_8;
use predawn_core::response::Response;
use rudi::Context;

pub use self::{rapidoc::RapiDoc, swagger_ui::SwaggerUI};
use crate::{
    config::{openapi::OpenAPIConfig, server::ServerConfig, Config},
    handler::{handler_fn, DynHandler},
    normalized_path::NormalizedPath,
};

pub(crate) fn create_route<F>(
    cx: &mut Context,
    get_path: F,
    html: String,
) -> (NormalizedPath, IndexMap<Method, DynHandler>)
where
    F: Fn(OpenAPIConfig) -> NormalizedPath,
{
    (get_path(cx.resolve::<OpenAPIConfig>()), create_map(html))
}

pub(crate) fn json_path(cfg: &Config) -> NormalizedPath {
    let server_cfg = ServerConfig::from(cfg);
    let openapi_cfg = OpenAPIConfig::from(cfg);

    let full_non_application_root_path = server_cfg.full_non_application_root_path();
    let normalized_json_path = openapi_cfg.json_path;

    full_non_application_root_path.join(normalized_json_path)
}

fn create_map(html: String) -> IndexMap<Method, DynHandler> {
    let handler = handler_fn(move |_| {
        let html = html.clone();

        async move {
            let mut response: Response = Response::new(html.into());
            response.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static(TEXT_HTML_UTF_8.as_ref()),
            );
            Ok(response)
        }
    });

    let handler = DynHandler::new(handler);

    let mut map = IndexMap::with_capacity(1);
    map.insert(Method::GET, handler);
    map
}
