use std::collections::{btree_map::Entry, BTreeMap};

use indexmap::IndexMap;
#[doc(inline)]
pub use openapiv3::*;

pub fn merge_responses(
    old: &mut BTreeMap<http::StatusCode, Response>,
    new: BTreeMap<http::StatusCode, Response>,
) {
    new.into_iter()
        .for_each(|(status, new)| match old.entry(status) {
            Entry::Occupied(mut old) => {
                let old = old.get_mut();
                old.headers.extend(new.headers);
                old.content.extend(new.content);
                old.links.extend(new.links);
                old.extensions.extend(new.extensions);
            }
            Entry::Vacant(old) => {
                old.insert(new);
            }
        });
}

pub trait ToParameters {
    fn parameters(components: &mut Components) -> Vec<ParameterData>;
}

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
) -> IndexMap<openapiv3::StatusCode, ReferenceOr<Response>> {
    responses
        .into_iter()
        .map(|(status, response)| {
            (
                openapiv3::StatusCode::Code(status.as_u16()),
                ReferenceOr::Item(response),
            )
        })
        .collect()
}
