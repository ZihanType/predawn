use std::collections::HashMap;

use futures_util::{future::Either, Future, FutureExt};
use http::Method;
use matchit::{InsertError, Match};
use predawn_core::{error::Error, request::Request, response::Response};

use crate::{
    handler::{DynHandler, Handler},
    path_params::PathParams,
    response_error::{MatchError, MethodNotAllowedError},
};

#[derive(Default)]
pub struct MethodRouter {
    methods: HashMap<Method, DynHandler>,
}

impl From<HashMap<Method, DynHandler>> for MethodRouter {
    fn from(methods: HashMap<Method, DynHandler>) -> Self {
        Self { methods }
    }
}

impl Handler for MethodRouter {
    fn call(&self, mut req: Request) -> impl Future<Output = Result<Response, Error>> + Send {
        let method = &mut req.head.method;

        match self.methods.get(method) {
            Some(handler) => Either::Left(handler.call(req)),
            None => Either::Right(
                if *method == Method::HEAD {
                    *method = Method::GET;

                    Either::Left(
                        async move {
                            let mut response = self.call(req).await?;
                            response.body_mut().clear();
                            Ok(response)
                        }
                        .boxed(),
                    )
                } else {
                    Either::Right(async { Err(MethodNotAllowedError.into()) })
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

        let matched = self.at(head.uri.path()).map_err(MatchError)?;

        head.extensions
            .get_or_insert_default::<PathParams>()
            .insert(matched.params);

        matched.value.call(req).await
    }
}
