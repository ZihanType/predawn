use std::{future::Future, sync::Arc};

use async_trait::async_trait;
use predawn_core::{
    error::Error, into_response::IntoResponse, request::Request, response::Response,
};

use crate::handler::Handler;

pub struct Around<H, F> {
    pub(crate) inner: Arc<H>,
    pub(crate) f: F,
}

#[async_trait]
impl<H, F, Fut, R> Handler for Around<H, F>
where
    H: Handler,
    F: Fn(Arc<H>, Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<R, Error>> + Send,
    R: IntoResponse,
{
    async fn call(&self, req: Request) -> Result<Response, Error> {
        match (self.f)(self.inner.clone(), req).await {
            Ok(r) => r.into_response().map_err(Into::into),
            Err(e) => Err(e),
        }
    }
}
