use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use http::StatusCode;

use crate::{
    error::BoxError,
    openapi::{self, merge_responses, Schema},
    response::Response,
    response_error::ResponseError,
};

#[derive(Debug)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> fmt::Display for Either<L, R>
where
    L: fmt::Display,
    R: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Either::Left(l) => fmt::Display::fmt(l, f),
            Either::Right(r) => fmt::Display::fmt(r, f),
        }
    }
}

impl<L, R> Error for Either<L, R>
where
    L: Error,
    R: Error,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Either::Left(l) => l.source(),
            Either::Right(r) => r.source(),
        }
    }
}

impl<L, R> ResponseError for Either<L, R>
where
    L: ResponseError,
    R: ResponseError,
{
    fn as_status(&self) -> StatusCode {
        match self {
            Either::Left(l) => l.as_status(),
            Either::Right(r) => r.as_status(),
        }
    }

    fn status_codes() -> BTreeSet<StatusCode> {
        let mut status_codes = L::status_codes();
        status_codes.extend(R::status_codes());
        status_codes
    }

    fn as_response(&self) -> Response {
        match self {
            Either::Left(l) => l.as_response(),
            Either::Right(r) => r.as_response(),
        }
    }

    fn responses(
        schemas: &mut BTreeMap<String, Schema>,
        schemas_in_progress: &mut Vec<String>,
    ) -> BTreeMap<StatusCode, openapi::Response> {
        let mut responses = L::responses(schemas, schemas_in_progress);
        merge_responses(&mut responses, R::responses(schemas, schemas_in_progress));
        responses
    }

    #[doc(hidden)]
    fn inner(self, type_name: &mut Vec<&'static str>) -> BoxError {
        type_name.push(std::any::type_name::<Self>());

        match self {
            Either::Left(l) => l.inner(type_name),
            Either::Right(r) => r.inner(type_name),
        }
    }
}
