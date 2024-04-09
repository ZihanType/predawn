mod limit;
#[cfg_attr(docsrs, doc(cfg(feature = "tower-compat")))]
#[cfg(feature = "tower-compat")]
mod tower_compat;
mod tracing;

#[cfg_attr(docsrs, doc(cfg(feature = "tower-compat")))]
#[cfg(feature = "tower-compat")]
pub use self::tower_compat::TowerLayerCompatExt;
pub use self::{
    limit::{RequestBodyLimit, RequestBodyLimitHandler},
    tracing::{Tracing, TracingHandler},
};
use crate::handler::Handler;

pub trait Middleware<H: Handler> {
    type Output: Handler;

    fn transform(self, input: H) -> Self::Output;
}
