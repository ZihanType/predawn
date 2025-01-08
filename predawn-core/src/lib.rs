pub mod api_request;
pub mod api_response;
pub mod body;
pub mod either;
pub mod error;
pub mod from_request;
pub mod into_response;
mod macros;
pub mod media_type;
pub mod openapi;
pub mod request;
pub mod response;
pub mod response_error;
pub use error2;

pub(crate) mod private {
    #[derive(Debug, Clone, Copy)]
    pub enum ViaRequestHead {}

    #[derive(Debug, Clone, Copy)]
    pub enum ViaRequest {}
}
