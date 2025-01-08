use std::{
    future::poll_fn,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use hyper::body::Incoming;
use predawn_core::{
    error::Error, into_response::IntoResponse, request::Request, response::Response,
};
use tower::{Layer, Service};

use super::Middleware;
use crate::handler::Handler;

pub trait TowerLayerCompatExt: Sized {
    fn compat(self) -> TowerLayerCompatMiddleware<Self> {
        TowerLayerCompatMiddleware(self)
    }
}

impl<L> TowerLayerCompatExt for L {}

pub struct TowerLayerCompatMiddleware<L>(L);

impl<H, L> Middleware<H> for TowerLayerCompatMiddleware<L>
where
    H: Handler,
    L: Layer<HandlerToService<H>>,
    L::Service: Service<http::Request<Incoming>> + Send + Sync + 'static,
    <L::Service as Service<http::Request<Incoming>>>::Future: Send,
    <L::Service as Service<http::Request<Incoming>>>::Response: IntoResponse,
    <L::Service as Service<http::Request<Incoming>>>::Error: Into<Error>,
{
    type Output = ServiceToHandler<L::Service>;

    fn transform(self, input: H) -> Self::Output {
        let svc = self.0.layer(HandlerToService(Arc::new(input)));
        ServiceToHandler(Mutex::new(svc))
    }
}

pub struct HandlerToService<H>(Arc<H>);

impl<H> Service<http::Request<Incoming>> for HandlerToService<H>
where
    H: Handler,
{
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
    type Response = Response;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<Incoming>) -> Self::Future {
        let handler = self.0.clone();

        let req = Request::try_from(req).expect("not found some element in request extensions");

        async move { handler.call(req).await }.boxed()
    }
}

pub struct ServiceToHandler<S>(Mutex<S>);

impl<S> Handler for ServiceToHandler<S>
where
    S: Service<http::Request<Incoming>> + Send + Sync + 'static,
    S::Response: IntoResponse,
    S::Error: Into<Error>,
    S::Future: Send,
{
    async fn call(&self, req: Request) -> Result<Response, Error> {
        let svc = &self.0;

        poll_fn(|cx| svc.lock().unwrap().poll_ready(cx))
            .await
            .map_err(|e| e.into())?;

        let fut = svc
            .lock()
            .unwrap()
            .call(http::Request::<Incoming>::from(req));

        Ok(fut.await.map_err(|e| e.into())?.into_response()?)
    }
}
