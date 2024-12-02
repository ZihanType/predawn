use std::future::Future;

use futures_util::{future::Either, FutureExt};
use http::Method;
use indexmap::IndexMap;
use matchit::{InsertError, Match};
use predawn_core::{error::Error, request::Request, response::Response};
use snafu::ResultExt;

use crate::{
    handler::{DynHandler, Handler},
    path_params::PathParams,
    response_error::{DecodePathToUtf8Snafu, MatchSnafu, MethodNotAllowedSnafu},
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
                    Either::Right(async { Err(MethodNotAllowedSnafu.build().into()) })
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

        let mut parts = Vec::new();
        split_path(head.uri.path(), &mut parts);

        let mut path = String::new();
        for part in parts {
            match part {
                Part::Slash => path.push('/'),
                Part::Str(s) => {
                    let s = percent_encoding::percent_decode(s.as_bytes())
                        .decode_utf8()
                        .map_err(|_| DecodePathToUtf8Snafu { path: s }.build())?;

                    path.push_str(&s);
                }
            }
        }

        let matched = self.at(&path).context(MatchSnafu)?;

        head.extensions
            .get_or_insert_default::<PathParams>()
            .insert(matched.params);

        matched.value.call(req).await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part<'a> {
    Slash,
    Str(&'a str),
}

fn split_path<'a>(path: &'a str, parts: &mut Vec<Part<'a>>) {
    if path == "/" {
        parts.push(Part::Slash);
        return;
    }

    match path.split_once('/') {
        None => parts.push(Part::Str(path)),
        Some((left, right)) => {
            if left.is_empty() {
                parts.push(Part::Slash);
            } else {
                split_path(left, parts);
            }

            if !left.is_empty() && !right.is_empty() {
                parts.push(Part::Slash);
            }

            if right.is_empty() {
                parts.push(Part::Slash);
            } else {
                split_path(right, parts);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_path() {
        let mut parts = Vec::new();

        {
            split_path("", &mut parts);

            assert_eq!(parts, vec![Part::Str("")]);

            parts.clear();
        }

        {
            split_path("/", &mut parts);

            assert_eq!(parts, vec![Part::Slash]);

            parts.clear();
        }

        {
            split_path("/foo/bar", &mut parts);

            assert_eq!(
                parts,
                vec![Part::Slash, Part::Str("foo"), Part::Slash, Part::Str("bar")]
            );

            parts.clear();
        }

        {
            split_path("//cfg/foo/bar", &mut parts);

            assert_eq!(
                parts,
                vec![
                    Part::Slash,
                    Part::Slash,
                    Part::Str("cfg"),
                    Part::Slash,
                    Part::Str("foo"),
                    Part::Slash,
                    Part::Str("bar")
                ]
            );

            parts.clear();
        }

        {
            split_path("//cfg/foo/ /bar//", &mut parts);

            assert_eq!(
                parts,
                vec![
                    Part::Slash,
                    Part::Slash,
                    Part::Str("cfg"),
                    Part::Slash,
                    Part::Str("foo"),
                    Part::Slash,
                    Part::Str(" "),
                    Part::Slash,
                    Part::Str("bar"),
                    Part::Slash,
                    Part::Slash
                ]
            );

            parts.clear();
        }
    }
}
