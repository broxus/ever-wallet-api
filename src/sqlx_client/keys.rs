use crate::models::key::Key;
use crate::prelude::ServiceError;
use crate::sqlx_client::SqlxClient;

impl SqlxClient {
    pub async fn get_key(&self, api_key: &str) -> Result<Key, ServiceError> {
        sqlx::query!(r#"SELECT * FROM api_service_key WHERE key = $1"#, &api_key)
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }
}

#[cfg(test)]
mod test {
    use crate::models::transaction_kind::TransactionKind;
    use crate::models::transactions::TransactionsSearch;
    use crate::models::transactions_ordering::TransactionsOrdering;
    use crate::sqlx_client::SqlxClient;
    use sqlx::PgPool;

    #[tokio::test]
    async fn test() {
        let pg_pool =
            PgPool::connect("postgresql://postgres:postgres@localhost:5432/ton_wallet_api_rs")
                .await
                .unwrap();
    }
}
