#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod app;
pub mod config;
#[doc(hidden)]
pub mod controller;
pub mod environment;
pub mod extract;
pub mod handler;
pub mod middleware;
pub mod normalized_path;
mod path_params;
#[doc(hidden)]
pub mod path_util;
pub mod payload;
pub mod plugin;
pub mod route;
pub mod server;
pub mod test_client;

#[doc(inline)]
pub use predawn_core::*;
#[doc(inline)]
pub use predawn_macro::{controller, ToParameters, ToSchema};
#[cfg_attr(docsrs, doc(cfg(feature = "schemars")))]
#[cfg(feature = "schemars")]
#[doc(inline)]
pub use predawn_schema::schemars_transform;
#[doc(inline)]
pub use predawn_schema::ToSchema;

#[doc(hidden)]
pub mod __internal {
    pub use http;
    pub use indexmap;
    pub use paste;
    pub use predawn_core;
    pub use predawn_schema;
    pub use rudi;
}
