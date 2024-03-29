mod openapi_json;
pub mod ui;

use std::{collections::HashMap, sync::Arc};

use http::Method;
use rudi::Context;

use crate::{handler::DynHandler, normalized_path::NormalizedPath};

pub trait Plugin {
    fn create_route(
        self: Arc<Self>,
        cx: &mut Context,
    ) -> (NormalizedPath, HashMap<Method, DynHandler>);
}
