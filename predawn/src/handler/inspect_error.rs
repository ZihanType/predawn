use std::marker::PhantomData;

use async_trait::async_trait;
use predawn_core::{error::Error, request::Request, response::Response};

use crate::handler::Handler;

pub struct InspectError<H, F, Err> {
    pub(crate) inner: H,
    pub(crate) f: F,
    pub(crate) _marker: PhantomData<Err>,
}

#[async_trait]
impl<H, F, Err> Handler for InspectError<H, F, Err>
where
    H: Handler,
    F: Fn(&Err) + Send + Sync + 'static,
    Err: std::error::Error + Send + Sync + 'static,
{
    async fn call(&self, req: Request) -> Result<Response, Error> {
        self.inner.call(req).await.inspect_err(|e| {
            if let Some(e) = e.downcast_ref::<Err>() {
                (self.f)(e);
            }
        })
    }
}
