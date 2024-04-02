#![cfg_attr(docsrs, feature(doc_cfg))]

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
pub mod route;
pub mod server;
pub mod test_client;
pub mod to_header_value;
mod to_parameters;

pub use predawn_core::{
    api_request, api_response, body, either, error, from_request, into_response,
    media_type::{MultiRequestMediaType, MultiResponseMediaType},
    request,
    response::{MultiResponse, SingleResponse},
    response_error,
};
pub use predawn_macro::{
    controller, MultiRequestMediaType, MultiResponse, MultiResponseMediaType, SingleResponse,
    ToParameters, ToSchema,
};
#[cfg_attr(docsrs, doc(cfg(feature = "schemars")))]
#[cfg(feature = "schemars")]
pub use predawn_schema::schemars_transform;
pub use predawn_schema::{component_id, ToSchema};
pub use to_parameters::ToParameters;

#[doc(hidden)]
pub mod __internal {
    pub use http;
    pub use indexmap;
    pub use paste;
    pub use rudi;
}
