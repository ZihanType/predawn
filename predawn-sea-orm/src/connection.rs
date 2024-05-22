use std::sync::Arc;

use sea_orm::{DatabaseConnection, TransactionTrait};

use crate::{Error, Transaction};

#[derive(Debug, Clone)]
pub struct Connection {
    name: Arc<str>,
    conn: DatabaseConnection,
    transactions: Vec<Transaction>,
}

impl Connection {
    pub(crate) fn new(name: Arc<str>, conn: DatabaseConnection) -> Self {
        Self {
            name,
            conn,
            transactions: Vec::new(),
        }
    }

    pub async fn current_txn(&mut self) -> Result<Transaction, Error> {
        match self.transactions.last() {
            Some(txn) => Ok(txn.clone()),
            None => self.new_txn().await,
        }
    }

    pub async fn new_txn(&mut self) -> Result<Transaction, Error> {
        let txn = self.conn.begin().await?;
        let txn = Transaction(Arc::new(txn));
        self.transactions.push(txn.clone());
        Ok(txn)
    }

    pub async fn commit(&mut self) -> Result<(), Error> {
        let Some(Transaction(txn)) = self.transactions.pop() else {
            return Err(Error::NoTransactionsToCommit {
                name: self.name.clone(),
            });
        };

        match Arc::try_unwrap(txn) {
            Ok(txn) => {
                txn.commit().await?;
                Ok(())
            }
            Err(txn) => {
                self.transactions.push(Transaction(txn));
                Err(Error::TransactionReferencesError {
                    name: self.name.clone(),
                })
            }
        }
    }

    pub async fn commit_all(&mut self) -> Result<(), Error> {
        while !self.transactions.is_empty() {
            self.commit().await?;
        }

        Ok(())
    }

    pub async fn rollback(&mut self) -> Result<(), Error> {
        let Some(Transaction(txn)) = self.transactions.pop() else {
            return Err(Error::NoTransactionsToRollback {
                name: self.name.clone(),
            });
        };

        match Arc::try_unwrap(txn) {
            Ok(txn) => {
                txn.rollback().await?;
                Ok(())
            }
            Err(txn) => {
                self.transactions.push(Transaction(txn));
                Err(Error::TransactionReferencesError {
                    name: self.name.clone(),
                })
            }
        }
    }

    pub async fn rollback_all(&mut self) -> Result<(), Error> {
        while !self.transactions.is_empty() {
            self.rollback().await?;
        }

        Ok(())
    }
}
