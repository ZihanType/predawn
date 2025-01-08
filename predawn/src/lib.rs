#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate self as predawn;

pub mod any_map;
pub mod app;
pub mod config;
#[doc(hidden)]
pub mod controller;
pub mod environment;
pub mod extract;
pub mod handler;
mod macros;
pub mod media_type;
pub mod middleware;
pub mod normalized_path;
pub mod openapi;
mod path_params;
pub mod payload;
pub mod plugin;
pub mod response;
pub mod response_error;
pub mod route;
pub mod server;
pub mod test_client;
mod traits;
pub(crate) mod util;
pub use error2;
pub use http;
pub use predawn_core::{
    api_request, api_response, body, either, error, from_request, into_response,
    media_type::{MultiRequestMediaType, MultiResponseMediaType},
    request,
    response::{MultiResponse, SingleResponse},
};
#[cfg_attr(docsrs, doc(cfg(feature = "macro")))]
#[cfg(feature = "macro")]
pub use predawn_macro::{
    controller, MultiRequestMediaType, MultiResponse, MultiResponseMediaType, SecurityScheme,
    SingleResponse, Tag, ToParameters,
};
#[cfg_attr(docsrs, doc(cfg(feature = "schemars")))]
#[cfg(feature = "schemars")]
pub use predawn_schema::schemars_transform;
pub use predawn_schema::to_schema::ToSchema;
#[cfg_attr(docsrs, doc(cfg(feature = "macro")))]
#[cfg(feature = "macro")]
pub use predawn_schema_macro::ToSchema;

pub use self::traits::{SecurityScheme, Tag, ToParameters};

#[doc(hidden)]
pub mod __internal {
    pub use indexmap;
    pub use paste;
    pub use rudi;
    pub use serde_json;
}
