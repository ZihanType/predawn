mod rapidoc;
mod swagger_ui;

use std::{collections::HashMap, sync::Arc};

use http::{header::CONTENT_TYPE, HeaderValue, Method};
use mime::TEXT_HTML_UTF_8;
use predawn_core::response::Response;
pub use rapidoc::RapiDoc;
use rudi::Context;
pub use swagger_ui::SwaggerUI;

use crate::{
    config::{openapi::OpenAPIConfig, server::ServerConfig, Config},
    handler::{handler_fn, Handler},
    normalized_path::NormalizedPath,
};

pub(crate) fn create_route<F>(
    cx: &mut Context,
    get_path: F,
    html: String,
) -> (NormalizedPath, HashMap<Method, Arc<dyn Handler>>)
where
    F: Fn(OpenAPIConfig) -> NormalizedPath,
{
    (get_path(cx.resolve::<OpenAPIConfig>()), create_map(html))
}

pub(crate) fn json_path(cfg: &Config) -> NormalizedPath {
    let server_cfg = cfg.get::<ServerConfig>().unwrap_or_default();
    let openapi_cfg = cfg.get::<OpenAPIConfig>().unwrap_or_default();

    let full_non_application_root_path = server_cfg.full_non_application_root_path();
    let normalized_json_path = openapi_cfg.normalized_json_path();

    full_non_application_root_path.join(normalized_json_path)
}

fn create_map(html: String) -> HashMap<Method, Arc<dyn Handler>> {
    let mut map = HashMap::with_capacity(1);

    let handler = handler_fn(move |_| {
        let html = html.clone();

        async move {
            let mut response = Response::new(html.into());
            response.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static(TEXT_HTML_UTF_8.as_ref()),
            );
            Ok(response)
        }
    });

    let handler: Arc<dyn Handler> = Arc::new(handler);

    map.insert(Method::GET, handler);

    map
}
