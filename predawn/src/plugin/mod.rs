mod openapi_json;
pub mod ui;

use std::sync::Arc;

use http::Method;
use indexmap::IndexMap;
use rudi::Context;

use crate::{handler::DynHandler, normalized_path::NormalizedPath};

pub trait Plugin {
    fn create_route(
        self: Arc<Self>,
        cx: &mut Context,
    ) -> (NormalizedPath, IndexMap<Method, DynHandler>);
}
