use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use http::Method;
use predawn_core::openapi::{Components, PathItem, ReferenceOr};
use rudi::Context;

use crate::{handler::Handler, normalized_path::NormalizedPath};

#[doc(hidden)]
pub trait Controller {
    fn insert_routes<'a>(
        self: Arc<Self>,
        cx: &'a mut Context,
        route_table: &'a mut HashMap<NormalizedPath, HashMap<Method, Arc<dyn Handler>>>,
        paths: &'a mut BTreeMap<NormalizedPath, ReferenceOr<PathItem>>,
        components: &'a mut Components,
    );
}
