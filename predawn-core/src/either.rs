use std::{
    collections::{BTreeMap, HashSet},
    error::Error,
    fmt,
};

use http::StatusCode;

use crate::{
    error::BoxError,
    openapi::{self, merge_responses, Components},
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

    fn status_codes() -> HashSet<StatusCode> {
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

    fn responses(components: &mut Components) -> BTreeMap<StatusCode, openapi::Response> {
        let mut responses = L::responses(components);
        merge_responses(&mut responses, R::responses(components));
        responses
    }

    #[doc(hidden)]
    fn inner(self) -> BoxError
    where
        Self: Sized,
    {
        match self {
            Either::Left(l) => Box::new(l),
            Either::Right(r) => Box::new(r),
        }
    }
}
