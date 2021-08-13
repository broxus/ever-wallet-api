use std::collections::HashMap;

use bigdecimal::BigDecimal;
use sqlx::postgres::PgArguments;
use sqlx::Arguments;
use sqlx::Row;

use crate::models::address::CreateAddressInDb;
use crate::models::sqlx::AddressDb;
use crate::sqlx_client::SqlxClient;

impl SqlxClient {
    pub async fn create_address(
        &self,
        payload: CreateAddressInDb,
    ) -> Result<AddressDb, anyhow::Error> {
        sqlx::query_as!(AddressDb,
                r#"INSERT INTO address
                (service_id, workchain_id, hex, base64url, public_key, private_key, account_type, custodians, confirmations, custodians_public_keys)
                VALUES ($1, $2, $3, $4, $5, $6, $7::twa_account_type, $8, $9, $10)
                RETURNING
                id, service_id as "service_id: _", workchain_id, hex, base64url, public_key, private_key, account_type as "account_type: _", custodians, confirmations, custodians_public_keys, balance, created_at, updated_at
"#,
                payload.service_id,
                payload.workchain_id,
                payload.hex,
                payload.base64url,
                payload.public_key,
                payload.private_key,
                payload.account_type,
                payload.custodians,
                payload.confirmations,
                payload.custodians_public_keys
            )
            .execute(&self.pool)
            .await
    }

    // pub async fn get_total_supply_by_root_address(
    //     &self,
    //     root_address: &str,
    // ) -> Result<BigDecimal, anyhow::Error> {
    //     sqlx::query!(
    //         r#"SELECT SUM(amount) FROM balances WHERE root_address = $1"#,
    //         root_address
    //     )
    //     .fetch_one(&self.pool)
    //     .await
    //     .map(|x| x.sum.unwrap_or_default())
    //     .map_err(anyhow::Error::new)
    // }
}
