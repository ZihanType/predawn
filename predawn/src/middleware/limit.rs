use predawn_core::{
    error::Error,
    request::{BodyLimit, Request},
    response::Response,
};

use super::Middleware;
use crate::{handler::Handler, response_error::RequestBodyLimitSnafu};

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
        let limit = self.limit;

        let limit = match content_length {
            Some(content_length) if content_length > limit => {
                return Err(RequestBodyLimitSnafu {
                    content_length,
                    limit,
                }
                .build()
                .into());
            }
            Some(content_length) => content_length,
            None => limit,
        };

        req.head.body_limit = BodyLimit(limit);

        self.inner.call(req).await
    }
}
