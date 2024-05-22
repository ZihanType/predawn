use std::sync::Arc;

use crate::{DataSources, Error, Transaction, DATA_SOURCES, DEFAULT_DATA_SOURCE_NAME};

pub async fn default_txn() -> Result<Transaction, Error> {
    data_sources()?.current_txn(DEFAULT_DATA_SOURCE_NAME).await
}

pub async fn current_txn(name: &str) -> Result<Transaction, Error> {
    data_sources()?.current_txn(name).await
}

pub async fn new_txn(name: &str) -> Result<Transaction, Error> {
    data_sources()?.new_txn(name).await
}

pub async fn commit(name: &str) -> Result<(), Error> {
    data_sources()?.commit(name).await
}

pub async fn rollback(name: &str) -> Result<(), Error> {
    data_sources()?.rollback(name).await
}

pub fn data_sources() -> Result<Arc<DataSources>, Error> {
    DATA_SOURCES
        .try_with(Arc::clone)
        .map_err(|_| Error::NotSetDataSourcesError)
}
