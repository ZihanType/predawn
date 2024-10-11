use std::{collections::BTreeSet, sync::Arc};

use http::StatusCode;
use predawn::{
    error_ext::{ErrorExt, NextError},
    location::Location,
    response_error::ResponseError,
};
use sea_orm::DbErr;
use snafu::Snafu;

use crate::Transaction;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("{source}"))]
    DbErr {
        #[snafu(implicit)]
        location: Location,
        source: DbErr,
    },

    #[snafu(display("not found a data source `{name}`"))]
    NotFoundDataSource {
        #[snafu(implicit)]
        location: Location,
        name: Box<str>,
    },

    #[snafu(display("not set data sources in the current context"))]
    NotSetDataSources {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("inconsistent data source and transaction, data source name: `{data_source_name}`, transaction name : `{transaction_name}`"))]
    InconsistentDataSourceAndTransaction {
        #[snafu(implicit)]
        location: Location,
        data_source_name: Arc<str>,
        transaction_name: Arc<str>,
        txn: Transaction,
    },

    #[snafu(display("transaction have more than one reference, data source name: `{data_source_name}`, transaction hierarchy: `{transaction_hierarchy}`"))]
    TransactionHaveMoreThanOneReference {
        #[snafu(implicit)]
        location: Location,
        data_source_name: Arc<str>,
        transaction_hierarchy: usize,
        txn: Transaction,
    },

    #[snafu(display("nested transaction have more than one reference, data source name: `{data_source_name}`, current transaction hierarchy: `{current_transaction_hierarchy}`, nested transaction hierarchy: `{nested_transaction_hierarchy}`"))]
    NestedTransactionHaveMoreThanOneReference {
        #[snafu(implicit)]
        location: Location,
        data_source_name: Arc<str>,
        current_transaction_hierarchy: usize,
        nested_transaction_hierarchy: usize,
        txn: Transaction,
    },
}

impl ErrorExt for Error {
    fn entry(&self) -> (Location, NextError<'_>) {
        match self {
            Error::DbErr { location, source } => (*location, NextError::Std(source)),

            Error::NotFoundDataSource { location, .. }
            | Error::NotSetDataSources { location }
            | Error::InconsistentDataSourceAndTransaction { location, .. }
            | Error::TransactionHaveMoreThanOneReference { location, .. }
            | Error::NestedTransactionHaveMoreThanOneReference { location, .. } => {
                (*location, NextError::None)
            }
        }
    }
}

impl ResponseError for Error {
    fn as_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::INTERNAL_SERVER_ERROR);
    }
}
