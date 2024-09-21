use std::{collections::BTreeSet, sync::Arc};

use http::StatusCode;
use predawn::{error_stack::ErrorStack, location::Location, response_error::ResponseError};
use sea_orm::DbErr;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("{source}"))]
    DbErr {
        #[snafu(implicit)]
        location: Location,
        source: DbErr,
    },

    #[snafu(display(
        "data source `{name}` has more than one strong reference to its transaction"
    ))]
    TransactionReferencesError {
        #[snafu(implicit)]
        location: Location,
        name: Arc<str>,
    },

    #[snafu(display("not found a data source `{name}`"))]
    NotFoundDataSourceError {
        #[snafu(implicit)]
        location: Location,
        name: Box<str>,
    },

    #[snafu(display("not set data sources in the current context"))]
    NotSetDataSourcesError {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("data source `{name}` no transactions to commit"))]
    NoTransactionsToCommit {
        #[snafu(implicit)]
        location: Location,
        name: Arc<str>,
    },

    #[snafu(display("data source `{name}` no transactions to rollback"))]
    NoTransactionsToRollback {
        #[snafu(implicit)]
        location: Location,
        name: Arc<str>,
    },
}

impl ResponseError for Error {
    fn as_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn status_codes(codes: &mut BTreeSet<StatusCode>) {
        codes.insert(StatusCode::INTERNAL_SERVER_ERROR);
    }

    fn error_stack(&self, stack: &mut ErrorStack) {
        match self {
            Error::DbErr { location, source } => {
                stack.push(self, location);
                stack.push_without_location(source);
            }
            Error::TransactionReferencesError { location, .. } => {
                stack.push(self, location);
            }
            Error::NotFoundDataSourceError { location, .. } => {
                stack.push(self, location);
            }
            Error::NotSetDataSourcesError { location } => {
                stack.push(self, location);
            }
            Error::NoTransactionsToCommit { location, .. } => {
                stack.push(self, location);
            }
            Error::NoTransactionsToRollback { location, .. } => {
                stack.push(self, location);
            }
        }
    }
}
