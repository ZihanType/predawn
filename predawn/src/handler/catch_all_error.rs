use predawn_core::{
    error::Error, into_response::IntoResponse, request::Request, response::Response,
};

use crate::handler::Handler;

pub struct CatchAllError<H, F> {
    pub(crate) inner: H,
    pub(crate) f: F,
}

impl<H, F, Fut, R> Handler for CatchAllError<H, F>
where
    H: Handler,
    F: Fn(Error) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = R> + Send,
    R: IntoResponse,
{
    async fn call(&self, req: Request) -> Result<Response, Error> {
        match self.inner.call(req).await {
            Ok(response) => Ok(response),
            Err(e) => (self.f)(e).await.into_response().map_err(Into::into),
        }
    }
}
