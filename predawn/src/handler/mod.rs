mod after;
mod around;
mod before;
mod catch_all_error;
mod catch_error;
mod inspect_all_error;
mod inspect_error;

use std::{future::Future, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use predawn_core::{
    error::Error, into_response::IntoResponse, request::Request, response::Response,
};

use self::{
    after::After, around::Around, before::Before, catch_all_error::CatchAllError,
    catch_error::CatchError, inspect_all_error::InspectAllError, inspect_error::InspectError,
};
use crate::middleware::Middleware;

#[async_trait]
pub trait Handler: Send + Sync + 'static {
    async fn call(&self, req: Request) -> Result<Response, Error>;
}

#[async_trait]
impl<T: Handler + ?Sized> Handler for Arc<T> {
    async fn call(&self, req: Request) -> Result<Response, Error> {
        self.as_ref().call(req).await
    }
}

pub fn handler_fn<F, Fut, R>(f: F) -> HandlerFn<F>
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<R, Error>> + Send,
    R: IntoResponse,
{
    HandlerFn(f)
}

pub struct HandlerFn<F>(F);

#[async_trait]
impl<F, Fut, R> Handler for HandlerFn<F>
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<R, Error>> + Send,
    R: IntoResponse,
{
    async fn call(&self, req: Request) -> Result<Response, Error> {
        match (self.0)(req).await {
            Ok(r) => r.into_response().map_err(Into::into),
            Err(e) => Err(e),
        }
    }
}

#[doc(hidden)]
pub fn assert_handler<T: Handler>(_: &T) {}

pub trait HandlerExt: Handler + Sized {
    fn with<M>(self, middleware: M) -> M::Output
    where
        M: Middleware<Self>,
    {
        middleware.transform(self)
    }

    fn before<F, Fut>(self, f: F) -> Before<Self, F>
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Request, Error>> + Send,
    {
        Before { inner: self, f }
    }

    fn after<F, Fut, R>(self, f: F) -> After<Self, F>
    where
        F: Fn(Result<Response, Error>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<R, Error>> + Send,
        R: IntoResponse,
    {
        After { inner: self, f }
    }

    fn around<F, Fut, R>(self, f: F) -> Around<Self, F>
    where
        F: Fn(Arc<Self>, Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<R, Error>> + Send,
        R: IntoResponse,
    {
        Around {
            inner: Arc::new(self),
            f,
        }
    }

    fn inspect_error<F, Err>(self, f: F) -> InspectError<Self, F, Err>
    where
        F: Fn(&Err) + Send + Sync + 'static,
        Err: std::error::Error + Send + Sync + 'static,
    {
        InspectError {
            inner: self,
            f,
            _marker: PhantomData,
        }
    }

    fn inspect_all_error<F>(self, f: F) -> InspectAllError<Self, F>
    where
        F: Fn(&Error) + Send + Sync + 'static,
    {
        InspectAllError { inner: self, f }
    }

    fn catch_error<F, Err, Fut, R>(self, f: F) -> CatchError<Self, F, Err>
    where
        F: Fn(Err) -> Fut + Send + Sync + 'static,
        Err: std::error::Error + Send + Sync + 'static,
        Fut: Future<Output = R> + Send,
        R: IntoResponse,
    {
        CatchError {
            inner: self,
            f,
            _marker: PhantomData,
        }
    }

    fn catch_all_error<F, Fut, R>(self, f: F) -> CatchAllError<Self, F>
    where
        F: Fn(Error) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send,
        R: IntoResponse,
    {
        CatchAllError { inner: self, f }
    }
}

impl<T: Handler> HandlerExt for T {}
