use std::{collections::BTreeMap, sync::Arc};

use http::Method;
use indexmap::IndexMap;
use predawn_core::openapi::{Operation, Schema, SecurityScheme, Tag};
use rudi::Context;

use crate::{handler::DynHandler, normalized_path::NormalizedPath};

#[doc(hidden)]
pub trait Controller {
    #[allow(clippy::too_many_arguments)]
    fn insert_routes(
        self: Arc<Self>,
        cx: &mut Context,
        route_table: &mut IndexMap<NormalizedPath, Vec<(Method, DynHandler)>>,
        paths: &mut IndexMap<NormalizedPath, Vec<(Method, Operation)>>,
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
        security_schemes: &mut BTreeMap<&'static str, (&'static str, SecurityScheme)>,
        tags: &mut BTreeMap<&'static str, (&'static str, Tag)>,
    );
}
