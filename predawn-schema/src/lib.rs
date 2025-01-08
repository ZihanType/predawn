#![cfg_attr(docsrs, feature(doc_cfg))]

mod impls;

#[cfg_attr(docsrs, doc(cfg(feature = "schemars")))]
#[cfg(feature = "schemars")]
mod schemars_transform;

#[doc(hidden)]
pub mod to_schema;

pub mod openapi {
    pub use openapiv3::*;
}
#[cfg_attr(docsrs, doc(cfg(feature = "macro")))]
#[cfg(feature = "macro")]
pub use predawn_schema_macro::ToSchema;

#[cfg_attr(docsrs, doc(cfg(feature = "schemars")))]
#[cfg(feature = "schemars")]
pub use self::schemars_transform::schemars_transform;
pub use self::to_schema::ToSchema;

#[doc(hidden)]
pub mod __internal {
    pub use serde_json;
}
