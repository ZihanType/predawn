use std::{collections::BTreeMap, sync::Arc};

use http::Method;
use predawn_core::openapi::{Operation, Schema, SecurityScheme, Tag};
use rudi::Context;

use crate::{handler::DynHandler, normalized_path::NormalizedPath};

#[doc(hidden)]
pub trait Controller {
    fn insert_routes<'a>(
        self: Arc<Self>,
        cx: &'a mut Context,
        route_table: &'a mut BTreeMap<NormalizedPath, Vec<(Method, DynHandler)>>,
        paths: &'a mut BTreeMap<NormalizedPath, Vec<(Method, Operation)>>,
        schemas: &'a mut BTreeMap<String, Schema>,
        security_schemes: &'a mut BTreeMap<&'static str, (&'static str, SecurityScheme)>,
        tags: &'a mut BTreeMap<&'static str, (&'static str, Tag)>,
    );
}
