use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use async_trait::async_trait;
use http::{Method, StatusCode};
use matchit::{InsertError, Match};
use predawn_core::{
    error::Error, request::Request, response::Response, response_error::ResponseError,
};

use crate::{handler::Handler, path_params::PathParams};

#[derive(Default)]
pub struct MethodRouter {
    methods: HashMap<Method, Arc<dyn Handler>>,
}

impl From<HashMap<Method, Arc<dyn Handler>>> for MethodRouter {
    fn from(methods: HashMap<Method, Arc<dyn Handler>>) -> Self {
        Self { methods }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("method not allowed")]
pub struct MethodNotAllowedError;

impl ResponseError for MethodNotAllowedError {
    fn as_status(&self) -> StatusCode {
        StatusCode::METHOD_NOT_ALLOWED
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::METHOD_NOT_ALLOWED].into()
    }
}

#[async_trait]
impl Handler for MethodRouter {
    async fn call(&self, mut req: Request) -> Result<Response, Error> {
        let method = &mut req.head.method;

        match self.methods.get(method) {
            Some(handler) => handler.call(req).await,
            None => {
                if *method == Method::HEAD {
                    *method = Method::GET;
                    let mut response = self.call(req).await?;
                    response.body_mut().clear();
                    return Ok(response);
                }

                Err(MethodNotAllowedError.into())
            }
        }
    }
}

#[derive(Default)]
pub struct Router {
    router: matchit::Router<MethodRouter>,
    routes: Vec<(Box<str>, Vec<Method>)>,
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
            let methods = method_router.methods.keys().cloned().collect::<Vec<_>>();
            router.router.insert(route.clone(), method_router)?;
            router.routes.push((route.into_boxed_str(), methods));
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

    pub fn routes(&self) -> &[(Box<str>, Vec<Method>)] {
        &self.routes
    }
}

#[async_trait]
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

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct MatchError(#[from] pub matchit::MatchError);

impl ResponseError for MatchError {
    fn as_status(&self) -> StatusCode {
        match self.0 {
            matchit::MatchError::NotFound => StatusCode::NOT_FOUND,
        }
    }

    fn status_codes() -> HashSet<StatusCode> {
        [StatusCode::NOT_FOUND].into()
    }
}
