use std::collections::BTreeMap;

use predawn_core::openapi::{ParameterData, Schema};

use crate::openapi;

pub trait ToParameters {
    fn parameters(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> Vec<ParameterData>;
}

pub trait Tag {
    const NAME: &'static str;

    fn create() -> openapi::Tag;
}

pub trait SecurityScheme {
    const NAME: &'static str;

    fn create() -> openapi::SecurityScheme;
}
