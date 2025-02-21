use std::sync::Arc;

use snafu::OptionExt;

use crate::{
    DATA_SOURCES, DEFAULT_DATA_SOURCE, DataSources, Transaction,
    error::{Error, NotFoundDataSourceSnafu, NotSetDataSourcesSnafu},
};

#[inline(always)]
pub async fn default_txn() -> Result<Transaction, Error> {
    current_txn(DEFAULT_DATA_SOURCE).await
}

pub async fn current_txn(name: &str) -> Result<Transaction, Error> {
    let txn = data_sources()?
        .get(name)
        .context(NotFoundDataSourceSnafu { name })?
        .current_txn()
        .await?;

    Ok(txn)
}

pub async fn create_txn(name: &str) -> Result<Transaction, Error> {
    let txn = data_sources()?
        .get(name)
        .context(NotFoundDataSourceSnafu { name })?
        .create_txn()
        .await?;

    Ok(txn)
}

pub async fn commit(txn: Transaction) -> Result<(), Error> {
    let data_sources = data_sources()?;

    let source = data_sources
        .get(&txn.name)
        .context(NotFoundDataSourceSnafu {
            name: txn.name.as_ref(),
        })?;

    source.commit(txn).await
}

pub async fn rollback(txn: Transaction) -> Result<(), Error> {
    let data_sources = data_sources()?;

    let source = data_sources
        .get(&txn.name)
        .context(NotFoundDataSourceSnafu {
            name: txn.name.as_ref(),
        })?;

    source.rollback(txn).await
}

pub fn data_sources() -> Result<Arc<DataSources>, Error> {
    DATA_SOURCES
        .try_with(Arc::clone)
        .map_err(|_| NotSetDataSourcesSnafu.build())
}
