use std::future::Future;

use predawn_core::{error::Error, request::Request, response::Response};

use crate::handler::Handler;

pub struct Before<H, F> {
    pub(crate) inner: H,
    pub(crate) f: F,
}

impl<H, F, Fut> Handler for Before<H, F>
where
    H: Handler,
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Request, Error>> + Send,
{
    async fn call(&self, req: Request) -> Result<Response, Error> {
        let req = (self.f)(req).await?;
        self.inner.call(req).await
    }
}
