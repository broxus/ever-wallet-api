use crate::models::*;
use crate::prelude::*;
use crate::sqlx_client::*;

impl SqlxClient {
    #[allow(dead_code)]
    pub async fn create_token_balances(
        &self,
        payload: CreateTokenBalanceInDb,
    ) -> Result<TokenBalanceFromDb, ServiceError> {
        sqlx::query_as!(TokenBalanceFromDb,
                r#"INSERT INTO token_balances
                (service_id, account_workchain_id, account_hex, balance, root_address)
                VALUES ($1, $2, $3, $4, $5)
                RETURNING
                service_id as "service_id: _", account_workchain_id, account_hex, balance, root_address, created_at, updated_at
"#,
                payload.service_id as ServiceId,
                payload.account_workchain_id,
                payload.account_hex,
                payload.balance,
                payload.root_address,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn get_token_balance(
        &self,
        service_id: ServiceId,
        account_workchain_id: i32,
        account_hex: String,
        root_address: String,
    ) -> Result<TokenBalanceFromDb, ServiceError> {
        sqlx::query_as!(TokenBalanceFromDb,
                r#"SELECT service_id as "service_id: _", account_workchain_id, account_hex, balance, root_address, created_at, updated_at
                FROM token_balances
                WHERE service_id = $1 AND account_workchain_id = $2 AND account_hex = $3 and root_address = $4"#,
                service_id as ServiceId,
                account_workchain_id,
                account_hex,
                root_address
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }
    #[allow(dead_code)]
    pub async fn get_token_balance_by_workchain_hex(
        &self,
        account_workchain_id: i32,
        account_hex: String,
        root_address: String,
    ) -> Result<TokenBalanceFromDb, ServiceError> {
        sqlx::query_as!(TokenBalanceFromDb,
                r#"SELECT service_id as "service_id: _", account_workchain_id, account_hex, balance, root_address, created_at, updated_at
                FROM token_balances
                WHERE account_workchain_id = $1 AND account_hex = $2 and root_address = $3"#,
                account_workchain_id,
                account_hex,
                root_address
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }
    pub async fn get_token_balances(
        &self,
        service_id: ServiceId,
        account_workchain_id: i32,
        account_hex: String,
    ) -> Result<Vec<TokenBalanceFromDb>, ServiceError> {
        sqlx::query_as!(TokenBalanceFromDb,
                r#"SELECT service_id as "service_id: _", account_workchain_id, account_hex, balance, root_address, created_at, updated_at
                FROM token_balances
                WHERE service_id = $1 AND account_workchain_id = $2 AND account_hex = $3 "#,
                service_id as ServiceId,
                account_workchain_id,
                account_hex
            )
            .fetch_all(&self.pool)
            .await
            .map_err(From::from)
    }
}
