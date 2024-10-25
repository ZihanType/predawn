mod config;
mod data_source;
mod data_sources;
mod error;
mod function;
mod inner;
mod middleware;
mod transaction;

pub const DEFAULT_DATA_SOURCE: &str = "default";

tokio::task_local! {
    pub static DATA_SOURCES: std::sync::Arc<DataSources>;
}

pub use self::{
    config::{ConnectOptions, DataSourcesConfig, SlowStatementsLoggingSettings},
    data_source::DataSource,
    data_sources::DataSources,
    error::Error,
    function::{commit, create_txn, current_txn, data_sources, default_txn, rollback},
    middleware::{SeaOrmHandler, SeaOrmMiddleware},
    transaction::Transaction,
};
