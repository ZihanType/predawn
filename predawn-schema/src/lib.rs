#![cfg_attr(docsrs, feature(doc_cfg))]

mod impls;
#[cfg_attr(docsrs, doc(cfg(feature = "schemars")))]
#[cfg(feature = "schemars")]
mod schemars_transform;
mod to_schema;

#[cfg_attr(docsrs, doc(cfg(feature = "schemars")))]
#[cfg(feature = "schemars")]
pub use schemars_transform::schemars_transform;
pub use to_schema::ToSchema;
