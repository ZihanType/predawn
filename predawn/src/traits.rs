use indexmap::IndexMap;
use predawn_core::openapi::{ParameterData, ReferenceOr, Schema};

use crate::openapi;

pub trait ToParameters {
    fn parameters(schemas: &mut IndexMap<String, ReferenceOr<Schema>>) -> Vec<ParameterData>;
}

pub trait Tag {
    const NAME: &'static str;

    fn create() -> openapi::Tag;
}

pub trait SecurityScheme {
    const NAME: &'static str;

    fn create() -> openapi::SecurityScheme;
}
