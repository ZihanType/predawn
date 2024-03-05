use std::sync::{Arc, Mutex};

use predawn_core::{error::Error, into_response::IntoResponse, request::Request};
use tower::{Layer, Service};

use self::private::{HandlerToService, ServiceToHandler};
use super::Middleware;
use crate::handler::Handler;

pub trait TowerLayerCompatExt {
    fn compat(self) -> TowerCompatMiddleware<Self>
    where
        Self: Sized,
    {
        TowerCompatMiddleware(self)
    }
}

impl<L> TowerLayerCompatExt for L {}

pub struct TowerCompatMiddleware<L>(L);

impl<H, L> Middleware<H> for TowerCompatMiddleware<L>
where
    H: Handler,
    L: Layer<HandlerToService<H>>,
    L::Service: Service<Request> + Send + Sync + 'static,
    <L::Service as Service<Request>>::Future: Send,
    <L::Service as Service<Request>>::Response: IntoResponse,
    <L::Service as Service<Request>>::Error: Into<Error>,
{
    type Output = ServiceToHandler<L::Service>;

    fn transform(self, input: H) -> Self::Output {
        let svc = self.0.layer(HandlerToService(Arc::new(input)));
        ServiceToHandler(Arc::new(Mutex::new(svc)))
    }
}

mod private {
    use std::{
        future::poll_fn,
        sync::{Arc, Mutex},
        task::{Context, Poll},
    };

    use async_trait::async_trait;
    use futures_util::{future::BoxFuture, FutureExt};
    use predawn_core::{
        error::Error, into_response::IntoResponse, request::Request, response::Response,
    };
    use tower::Service;

    use crate::handler::Handler;

    pub struct HandlerToService<H>(pub Arc<H>);

    impl<H> Service<Request> for HandlerToService<H>
    where
        H: Handler,
    {
        type Error = Error;
        type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
        type Response = Response;

        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: Request) -> Self::Future {
            let handler = self.0.clone();
            async move { handler.call(req).await }.boxed()
        }
    }

    pub struct ServiceToHandler<S>(pub Arc<Mutex<S>>);

    #[async_trait]
    impl<S> Handler for ServiceToHandler<S>
    where
        S: Service<Request> + Send + Sync + 'static,
        S::Response: IntoResponse,
        S::Error: Into<Error>,
        S::Future: Send,
    {
        async fn call(&self, req: Request) -> Result<Response, Error> {
            let svc = self.0.clone();

            poll_fn(|cx| svc.lock().unwrap().poll_ready(cx))
                .await
                .map_err(Into::into)?;

            let fut = svc.lock().unwrap().call(req);

            Ok(fut.await.map_err(Into::into)?.into_response()?)
        }
    }
}
