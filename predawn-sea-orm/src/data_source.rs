use std::sync::Arc;

use sea_orm::{DatabaseConnection, TransactionTrait};
use snafu::ResultExt;
use tokio::sync::Mutex;

use crate::{
    Error, Transaction,
    error::{
        DbErrSnafu, InconsistentDataSourceAndTransactionSnafu,
        NestedTransactionHaveMoreThanOneReferenceSnafu, TransactionHaveMoreThanOneReferenceSnafu,
    },
};

#[derive(Debug)]
pub struct DataSource {
    name: Arc<str>,
    connection: DatabaseConnection,
    transactions: Mutex<Vec<Transaction>>,
}

impl DataSource {
    pub(crate) fn new(name: Arc<str>, conn: DatabaseConnection) -> Self {
        Self {
            name,
            connection: conn,
            transactions: Default::default(),
        }
    }

    pub async fn current_txn(&self) -> Result<Transaction, Error> {
        {
            let transactions = self.transactions.lock().await;

            if let Some(transaction) = transactions.last() {
                return Ok(transaction.clone());
            }
        }

        self.create_txn().await
    }

    pub async fn create_txn(&self) -> Result<Transaction, Error> {
        let mut transactions = self.transactions.lock().await;

        let result = match transactions.last() {
            Some(txn) => txn.begin().await,
            None => self.connection.begin().await,
        };

        let transaction = result.context(DbErrSnafu)?;

        let transaction = Transaction {
            name: self.name.clone(),
            inner: Arc::new(transaction),
            index: transactions.len(),
        };

        transactions.push(transaction.clone());

        Ok(transaction)
    }

    pub fn standalone(&self) -> Self {
        Self {
            name: self.name.clone(),
            connection: self.connection.clone(),
            transactions: Default::default(),
        }
    }
}

macro_rules! single_operation {
    ($ident:ident) => {
        pub async fn $ident(&self, txn: Transaction) -> Result<(), Error> {
            if self.name != txn.name {
                return InconsistentDataSourceAndTransactionSnafu {
                    data_source_name: self.name.clone(),
                    transaction_name: txn.name.clone(),
                    txn,
                }
                .fail();
            }

            {
                let mut transactions = self.transactions.lock().await;

                debug_assert!(txn.index < transactions.len());

                for _ in txn.index..transactions.len() {
                    let Transaction { name, inner, index } = transactions.pop().unwrap();

                    if index == txn.index {
                        drop(txn);

                        match Arc::try_unwrap(inner) {
                            Ok(txn) => {
                                txn.$ident().await.context(DbErrSnafu)?;
                                return Ok(());
                            }
                            Err(inner) => {
                                let last = Transaction { name, inner, index };
                                let txn = last.clone();

                                transactions.push(last);

                                return TransactionHaveMoreThanOneReferenceSnafu {
                                    data_source_name: self.name.clone(),
                                    transaction_hierarchy: index,
                                    txn,
                                }
                                .fail();
                            }
                        }
                    } else {
                        match Arc::try_unwrap(inner) {
                            Ok(txn) => {
                                txn.$ident().await.context(DbErrSnafu)?;
                            }
                            Err(inner) => {
                                let last = Transaction { name, inner, index };

                                transactions.push(last);

                                return NestedTransactionHaveMoreThanOneReferenceSnafu {
                                    data_source_name: self.name.clone(),
                                    current_transaction_hierarchy: txn.index,
                                    nested_transaction_hierarchy: index,
                                    txn,
                                }
                                .fail();
                            }
                        }
                    }
                }
            }

            Ok(())
        }
    };
}

macro_rules! multi_operation {
    ($multi:ident, $single:ident) => {
        pub async fn $multi(&self) -> Result<(), Error> {
            let mut transactions = self.transactions.lock().await;

            while let Some(Transaction { name, inner, index }) = transactions.pop() {
                match Arc::try_unwrap(inner) {
                    Ok(txn) => {
                        txn.$single().await.context(DbErrSnafu)?;
                    }
                    Err(inner) => {
                        let last = Transaction { name, inner, index };

                        transactions.push(last.clone());

                        return TransactionHaveMoreThanOneReferenceSnafu {
                            data_source_name: self.name.clone(),
                            transaction_hierarchy: index,
                            txn: last,
                        }
                        .fail();
                    }
                }
            }

            Ok(())
        }
    };
}

impl DataSource {
    single_operation!(commit);

    single_operation!(rollback);

    multi_operation!(commit_all, commit);

    multi_operation!(rollback_all, rollback);
}
