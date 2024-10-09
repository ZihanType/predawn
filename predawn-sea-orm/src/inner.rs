use std::{collections::HashMap, sync::Arc};

use rudi::Singleton;
use sea_orm::{Database, DatabaseConnection};

use crate::DataSourcesConfig;

#[derive(Debug, Clone)]
pub(crate) struct Inner(pub(crate) HashMap<Arc<str>, DatabaseConnection>);

#[Singleton]
impl Inner {
    #[di]
    async fn new(cfg: DataSourcesConfig) -> Self {
        let DataSourcesConfig { data_sources } = cfg;

        let mut map = HashMap::with_capacity(data_sources.len());

        for (name, options) in data_sources {
            let conn = Database::connect(options)
                .await
                .expect("failed to create data sources");

            let _ = map.insert(name.into(), conn);
        }

        Self(map)
    }
}
