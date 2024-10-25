use std::{collections::HashMap, sync::Arc};

use predawn::{
    error::Error, handler::Handler, middleware::Middleware, request::Request, response::Response,
};
use rudi::Transient;
use sea_orm::DatabaseConnection;

use crate::{inner::Inner, DataSources, DATA_SOURCES, DEFAULT_DATA_SOURCE};

#[derive(Debug, Clone)]
pub struct SeaOrmMiddleware {
    data_sources: HashMap<Arc<str>, DatabaseConnection>,
}

impl SeaOrmMiddleware {
    pub fn with_default(conn: DatabaseConnection) -> Self {
        let mut data_sources = HashMap::new();

        data_sources.insert(Arc::<str>::from(DEFAULT_DATA_SOURCE), conn);

        Self { data_sources }
    }

    pub fn new(map: HashMap<Arc<str>, DatabaseConnection>) -> Self {
        Self { data_sources: map }
    }
}

#[Transient]
impl SeaOrmMiddleware {
    #[di]
    async fn inject(Inner(map): Inner) -> Self {
        Self { data_sources: map }
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
    data_sources: HashMap<Arc<str>, DatabaseConnection>,
    inner: H,
}

impl<H: Handler> Handler for SeaOrmHandler<H> {
    async fn call(&self, req: Request) -> Result<Response, Error> {
        let data_sources = Arc::new(DataSources::new(&self.data_sources));

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
