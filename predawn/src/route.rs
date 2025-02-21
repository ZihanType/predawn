use futures_util::{FutureExt, future::Either};
use http::Method;
use indexmap::IndexMap;
use matchit::{InsertError, Match};
use predawn_core::{error::Error, request::Request, response::Response};
use snafu::ResultExt;

use crate::{
    handler::{DynHandler, Handler},
    path_params::PathParams,
    response_error::{MatchSnafu, MethodNotAllowedSnafu},
};

#[derive(Default)]
pub struct MethodRouter {
    methods: IndexMap<Method, DynHandler>,
}

impl From<IndexMap<Method, DynHandler>> for MethodRouter {
    fn from(methods: IndexMap<Method, DynHandler>) -> Self {
        Self { methods }
    }
}

impl Handler for MethodRouter {
    fn call(&self, mut req: Request) -> impl Future<Output = Result<Response, Error>> + Send {
        let method = &mut req.head.method;

        match self.methods.get(method) {
            Some(handler) => Either::Left(handler.call(req)),
            None => Either::Right(
                if *method != Method::HEAD {
                    Either::Left(async { Err(MethodNotAllowedSnafu.build().into()) })
                } else {
                    *method = Method::GET;

                    Either::Right(
                        async move {
                            let mut response = self.call(req).await?;
                            response.body_mut().clear();
                            Ok(response)
                        }
                        .boxed(),
                    )
                },
            ),
        }
    }
}

#[derive(Default)]
pub struct Router {
    router: matchit::Router<MethodRouter>,
    routes: Vec<(Box<str>, Box<[Method]>)>,
}

impl Router {
    pub fn insert<S>(&mut self, route: S, method_router: MethodRouter) -> Result<(), InsertError>
    where
        S: Into<String>,
    {
        fn inner_insert(
            router: &mut Router,
            route: String,
            method_router: MethodRouter,
        ) -> Result<(), InsertError> {
            let methods = method_router.methods.keys().cloned().collect();

            router.router.insert(route.clone(), method_router)?;
            router.routes.push((route.into(), methods));

            Ok(())
        }

        inner_insert(self, route.into(), method_router)
    }

    pub fn at<'m, 'p>(
        &'m self,
        path: &'p str,
    ) -> Result<Match<'m, 'p, &'m MethodRouter>, matchit::MatchError> {
        self.router.at(path)
    }

    pub fn routes(&self) -> &[(Box<str>, Box<[Method]>)] {
        &self.routes
    }
}

impl Handler for Router {
    async fn call(&self, mut req: Request) -> Result<Response, Error> {
        let head = &mut req.head;

        let matched = self.at(head.uri.path()).context(MatchSnafu)?;

        #[allow(unused_variables)]
        let prev = head.extensions.insert(PathParams::new(matched.params));
        debug_assert!(prev.is_none());

        matched.value.call(req).await
    }
}
