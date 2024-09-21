use predawn_core::{
    error::Error,
    location::Location,
    request::{BodyLimit, Request},
    response::Response,
    response_error::RequestBodyLimitError,
};

use super::Middleware;
use crate::handler::Handler;

pub struct RequestBodyLimit {
    limit: usize,
}

impl RequestBodyLimit {
    pub fn new(limit: usize) -> Self {
        Self { limit }
    }
}

impl<H: Handler> Middleware<H> for RequestBodyLimit {
    type Output = RequestBodyLimitHandler<H>;

    fn transform(self, input: H) -> Self::Output {
        RequestBodyLimitHandler {
            limit: self.limit,
            inner: input,
        }
    }
}

pub struct RequestBodyLimitHandler<H> {
    limit: usize,
    inner: H,
}

impl<H: Handler> Handler for RequestBodyLimitHandler<H> {
    async fn call(&self, mut req: Request) -> Result<Response, Error> {
        let content_length = req.head.content_length();

        let limit = match content_length {
            Some(len) => {
                if len > self.limit {
                    return Err(RequestBodyLimitError {
                        location: Location::caller(),
                        actual: Some(len),
                        expected: self.limit,
                    }
                    .into());
                } else {
                    len
                }
            }
            None => self.limit,
        };

        req.head.body_limit = BodyLimit(limit);

        self.inner.call(req).await
    }
}
