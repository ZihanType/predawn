use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{
    ConnectionTrait, DatabaseTransaction, DbBackend, DbErr, ExecResult, QueryResult, Statement,
    TransactionTrait,
};

#[derive(Debug, Clone)]
pub struct Transaction {
    pub(crate) name: Arc<str>,
    pub(crate) inner: Arc<DatabaseTransaction>,
    pub(crate) index: usize,
}

impl Transaction {
    #[inline]
    pub(crate) async fn begin(&self) -> Result<DatabaseTransaction, DbErr> {
        self.inner.begin().await
    }
}

#[async_trait]
impl ConnectionTrait for Transaction {
    fn get_database_backend(&self) -> DbBackend {
        self.inner.get_database_backend()
    }

    async fn execute(&self, stmt: Statement) -> Result<ExecResult, DbErr> {
        self.inner.execute(stmt).await
    }

    async fn execute_unprepared(&self, sql: &str) -> Result<ExecResult, DbErr> {
        self.inner.execute_unprepared(sql).await
    }

    async fn query_one(&self, stmt: Statement) -> Result<Option<QueryResult>, DbErr> {
        self.inner.query_one(stmt).await
    }

    async fn query_all(&self, stmt: Statement) -> Result<Vec<QueryResult>, DbErr> {
        self.inner.query_all(stmt).await
    }

    fn support_returning(&self) -> bool {
        self.inner.support_returning()
    }

    fn is_mock_connection(&self) -> bool {
        self.inner.is_mock_connection()
    }
}
