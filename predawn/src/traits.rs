use predawn_core::openapi::{Components, ParameterData};

use crate::openapi;

pub trait ToParameters {
    fn parameters(components: &mut Components) -> Vec<ParameterData>;
}

pub trait Tag {
    fn name() -> &'static str;

    fn create() -> openapi::Tag;
}
