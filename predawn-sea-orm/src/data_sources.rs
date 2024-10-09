use std::{collections::HashMap, sync::Arc};

use rudi::SingleOwner;
use sea_orm::DatabaseConnection;

use crate::{inner::Inner, DataSource, Error, DEFAULT_DATA_SOURCE};

#[derive(Debug)]
pub struct DataSources(HashMap<Arc<str>, DataSource>);

impl DataSources {
    pub fn with_default(conn: DatabaseConnection) -> Self {
        let name = Arc::<str>::from(DEFAULT_DATA_SOURCE);

        let mut map = HashMap::new();
        map.insert(name.clone(), DataSource::new(name, conn));

        Self(map)
    }

    pub fn new(map: HashMap<Arc<str>, DatabaseConnection>) -> Self {
        let map = map
            .into_iter()
            .map(|(name, conn)| (name.clone(), DataSource::new(name, conn)))
            .collect();

        Self(map)
    }

    pub fn get(&self, name: &str) -> Option<&DataSource> {
        self.0.get(name)
    }

    pub fn standalone(&self) -> Self {
        let map = self
            .0
            .iter()
            .map(|(name, conn)| (name.clone(), conn.standalone()))
            .collect();

        Self(map)
    }

    pub async fn commit_all(&self) -> Result<(), Error> {
        for source in self.0.values() {
            source.commit_all().await?;
        }

        Ok(())
    }

    pub async fn rollback_all(&self) -> Result<(), Error> {
        for source in self.0.values() {
            source.rollback_all().await?;
        }

        Ok(())
    }
}

#[SingleOwner]
impl DataSources {
    #[di]
    async fn inject(Inner(map): Inner) -> Self {
        Self::new(map)
    }
}
