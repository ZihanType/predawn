use predawn_core::openapi::{Components, ParameterData};

pub trait ToParameters {
    fn parameters(components: &mut Components) -> Vec<ParameterData>;
}
