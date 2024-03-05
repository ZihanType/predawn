use std::future::Future;

use async_trait::async_trait;
use predawn_core::{
    error::Error, into_response::IntoResponse, request::Request, response::Response,
};

use crate::handler::Handler;

pub struct After<H, F> {
    pub(crate) inner: H,
    pub(crate) f: F,
}

#[async_trait]
impl<H, F, Fut, R> Handler for After<H, F>
where
    H: Handler,
    F: Fn(Result<Response, Error>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<R, Error>> + Send,
    R: IntoResponse,
{
    async fn call(&self, req: Request) -> Result<Response, Error> {
        let result = self.inner.call(req).await;
        match (self.f)(result).await {
            Ok(r) => r.into_response().map_err(Into::into),
            Err(e) => Err(e),
        }
    }
}
