use crate::handler::Handler;

pub mod tower_compat;
pub mod tracing;

pub trait Middleware<H: Handler> {
    type Output: Handler;

    fn transform(self, input: H) -> Self::Output;
}
