use std::collections::{btree_map::Entry, BTreeMap};

pub use openapiv3::*;

#[doc(hidden)]
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
