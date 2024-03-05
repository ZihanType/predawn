use std::{future::Future, marker::PhantomData};

use async_trait::async_trait;
use predawn_core::{
    error::Error, into_response::IntoResponse, request::Request, response::Response,
};

use crate::handler::Handler;

pub struct CatchError<H, F, Err> {
    pub(crate) inner: H,
    pub(crate) f: F,
    pub(crate) _marker: PhantomData<Err>,
}

#[async_trait]
impl<H, F, Err, Fut, R> Handler for CatchError<H, F, Err>
where
    H: Handler,
    F: Fn(Err) -> Fut + Send + Sync + 'static,
    Err: std::error::Error + Send + Sync + 'static,
    Fut: Future<Output = R> + Send,
    R: IntoResponse,
{
    async fn call(&self, req: Request) -> Result<Response, Error> {
        match self.inner.call(req).await {
            Ok(response) => Ok(response),
            Err(e) => match e.downcast::<Err>() {
                Ok(e) => (self.f)(e).await.into_response().map_err(Into::into),
                Err(e) => Err(e),
            },
        }
    }
}
