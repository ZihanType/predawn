use crate::handler::Handler;

#[cfg_attr(docsrs, doc(cfg(feature = "tower-compat")))]
#[cfg(feature = "tower-compat")]
pub mod tower_compat;
pub mod tracing;

pub trait Middleware<H: Handler> {
    type Output: Handler;

    fn transform(self, input: H) -> Self::Output;
}
