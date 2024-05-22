use std::sync::Arc;

use rudi::Singleton;
use scc::HashMap;
use sea_orm::{Database, DatabaseConnection, DbErr};

use crate::{Connection, DataSourcesConfig, Error, Transaction, DEFAULT_DATA_SOURCE_NAME};

#[derive(Debug, Default, Clone)]
pub struct DataSources(HashMap<Arc<str>, Connection>);

impl DataSources {
    pub async fn with_default(conn: DatabaseConnection) -> Self {
        let name = Arc::<str>::from(DEFAULT_DATA_SOURCE_NAME);

        let map = HashMap::new();

        let _ = map
            .insert_async(name.clone(), Connection::new(name, conn))
            .await;

        Self(map)
    }

    pub async fn insert<N: Into<Arc<str>>>(
        &self,
        name: N,
        conn: DatabaseConnection,
    ) -> Result<(), (Arc<str>, Connection)> {
        self._insert(name.into(), conn).await
    }

    async fn _insert(
        &self,
        name: Arc<str>,
        conn: DatabaseConnection,
    ) -> Result<(), (Arc<str>, Connection)> {
        self.0
            .insert_async(name.clone(), Connection::new(name, conn))
            .await
    }
}

macro_rules! single_operation {
    ($ident:ident, $ty:ty) => {
        pub async fn $ident(&self, name: &str) -> Result<$ty, Error> {
            match self.0.get_async(name).await {
                Some(mut entry) => entry.get_mut().$ident().await,
                None => Err(Error::NotFoundDataSourceError { name: name.into() }),
            }
        }
    };
}

macro_rules! multi_operation {
    ($ident:ident) => {
        pub async fn $ident(&self) -> Result<(), Error> {
            let mut option_entry = self.0.first_entry_async().await;

            while let Some(mut entry) = option_entry {
                entry.get_mut().$ident().await?;
                option_entry = entry.next_async().await;
            }

            Ok(())
        }
    };
}

impl DataSources {
    single_operation!(current_txn, Transaction);

    single_operation!(new_txn, Transaction);

    single_operation!(commit, ());

    single_operation!(rollback, ());

    multi_operation!(commit_all);

    multi_operation!(rollback_all);
}

#[Singleton]
impl DataSources {
    #[di]
    async fn create(cfg: DataSourcesConfig) -> Self {
        Self::_create(cfg)
            .await
            .expect("failed to create `DataSources`")
    }

    async fn _create(cfg: DataSourcesConfig) -> Result<Self, DbErr> {
        let DataSourcesConfig {
            default,
            data_sources,
        } = cfg;

        let this = match default {
            Some(url) => {
                let conn = Database::connect(url).await?;
                Self::with_default(conn).await
            }
            None => Self::default(),
        };

        for (name, url) in data_sources {
            let conn = Database::connect(url).await?;
            let _ = this.insert(name, conn).await;
        }

        Ok(this)
    }
}
