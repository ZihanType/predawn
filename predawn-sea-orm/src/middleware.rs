use std::sync::Arc;

use predawn::{
    error::Error, handler::Handler, middleware::Middleware, request::Request, response::Response,
};
use rudi::Singleton;

use crate::{DataSources, DATA_SOURCES};

#[Singleton(async)]
#[derive(Debug, Clone)]
pub struct SeaOrmMiddleware {
    data_sources: DataSources,
}

impl SeaOrmMiddleware {
    pub fn new(data_sources: DataSources) -> Self {
        Self { data_sources }
    }
}

impl<H: Handler> Middleware<H> for SeaOrmMiddleware {
    type Output = SeaOrmHandler<H>;

    fn transform(self, input: H) -> Self::Output {
        SeaOrmHandler {
            data_sources: self.data_sources,
            inner: input,
        }
    }
}

pub struct SeaOrmHandler<H> {
    data_sources: DataSources,
    inner: H,
}

impl<H: Handler> Handler for SeaOrmHandler<H> {
    async fn call(&self, req: Request) -> Result<Response, Error> {
        let data_sources = Arc::new(self.data_sources.clone());

        let result = DATA_SOURCES
            .scope(data_sources.clone(), async { self.inner.call(req).await })
            .await;

        match &result {
            Ok(response) => {
                if response.status().is_success() {
                    data_sources.commit_all().await?;
                } else {
                    data_sources.rollback_all().await?;
                }
            }
            Err(_) => {
                data_sources.rollback_all().await?;
            }
        }

        result
    }
}
