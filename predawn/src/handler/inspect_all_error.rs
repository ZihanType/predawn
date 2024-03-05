use async_trait::async_trait;
use predawn_core::{error::Error, request::Request, response::Response};

use crate::handler::Handler;

pub struct InspectAllError<H, F> {
    pub(crate) inner: H,
    pub(crate) f: F,
}

#[async_trait]
impl<H, F> Handler for InspectAllError<H, F>
where
    H: Handler,
    F: Fn(&Error) + Send + Sync + 'static,
{
    async fn call(&self, req: Request) -> Result<Response, Error> {
        self.inner.call(req).await.inspect_err(|e| {
            (self.f)(e);
        })
    }
}
