use std::{collections::BTreeMap, sync::Arc};

use http::Method;
use predawn_core::openapi::{Components, Operation, Tag};
use rudi::Context;

use crate::{handler::DynHandler, normalized_path::NormalizedPath};

#[doc(hidden)]
pub trait Controller {
    fn insert_routes<'a>(
        self: Arc<Self>,
        cx: &'a mut Context,
        route_table: &'a mut BTreeMap<NormalizedPath, Vec<(Method, DynHandler)>>,
        paths: &'a mut BTreeMap<NormalizedPath, Vec<(Method, Operation)>>,
        components: &'a mut Components,
        tags: &'a mut BTreeMap<&'static str, (&'static str, Tag)>,
    );
}
