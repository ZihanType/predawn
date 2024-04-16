use std::{collections::BTreeMap, sync::Arc};

use http::Method;
use indexmap::IndexMap;
use predawn_core::openapi::{Components, PathItem, ReferenceOr};
use rudi::Context;

use crate::{handler::DynHandler, normalized_path::NormalizedPath};

#[doc(hidden)]
pub trait Controller {
    fn insert_routes<'a>(
        self: Arc<Self>,
        cx: &'a mut Context,
        route_table: &'a mut BTreeMap<NormalizedPath, IndexMap<Method, DynHandler>>,
        paths: &'a mut BTreeMap<NormalizedPath, ReferenceOr<PathItem>>,
        components: &'a mut Components,
    );
}
