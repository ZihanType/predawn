use std::collections::BTreeMap;

use indexmap::IndexMap;
pub use predawn_core::openapi::*;

#[doc(hidden)]
pub fn transform_parameters(parameters: Vec<Parameter>) -> Vec<ReferenceOr<Parameter>> {
    parameters.into_iter().map(ReferenceOr::Item).collect()
}

#[doc(hidden)]
pub fn transform_request_body(
    request_body: Option<RequestBody>,
) -> Option<ReferenceOr<RequestBody>> {
    request_body.map(ReferenceOr::Item)
}

#[doc(hidden)]
pub fn transform_responses(
    responses: BTreeMap<http::StatusCode, Response>,
) -> IndexMap<StatusCode, ReferenceOr<Response>> {
    responses
        .into_iter()
        .map(|(status, response)| {
            (
                StatusCode::Code(status.as_u16()),
                ReferenceOr::Item(response),
            )
        })
        .collect()
}
