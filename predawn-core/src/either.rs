use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use error2::{ErrorExt, Location, NextError};
use http::StatusCode;

use crate::{
    error::BoxError,
    openapi::{self, Schema, merge_responses},
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

impl<L, R> ErrorExt for Either<L, R>
where
    L: ErrorExt,
    R: ErrorExt,
{
    fn entry(&self) -> (Location, NextError<'_>) {
        match self {
            Either::Left(l) => l.entry(),
            Either::Right(r) => r.entry(),
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

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        L::status_codes(codes);
        R::status_codes(codes);
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
    fn inner(self) -> BoxError {
        match self {
            Either::Left(l) => l.inner(),
            Either::Right(r) => r.inner(),
        }
    }
}
