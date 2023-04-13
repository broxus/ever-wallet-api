use anyhow::Result;
use bigdecimal::BigDecimal;

use crate::models::*;
use crate::sqlx_client::*;

impl SqlxClient {
    pub async fn create_address(&self, payload: CreateAddressInDb) -> Result<AddressDb> {
        sqlx::query_as!(AddressDb,
                r#"INSERT INTO address
                (id, service_id, workchain_id, hex, base64url, public_key, private_key, account_type, custodians, confirmations, custodians_public_keys)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8::twa_account_type, $9, $10, $11)
                RETURNING
                id, service_id as "service_id: _", workchain_id, hex, base64url, public_key, private_key, account_type as "account_type: _", custodians, confirmations, custodians_public_keys, balance, created_at, updated_at
"#,
                payload.id,
                payload.service_id as ServiceId,
                payload.workchain_id,
                payload.hex,
                payload.base64url,
                payload.public_key,
                payload.private_key,
                payload.account_type as AccountType,
                payload.custodians,
                payload.confirmations,
                payload.custodians_public_keys
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn create_address_with_balance(
        &self,
        payload: CreateAddressInDb,
        balance: BigDecimal,
    ) -> Result<AddressDb> {
        sqlx::query_as!(AddressDb,
                r#"INSERT INTO address
                (id, service_id, workchain_id, hex, base64url, public_key, private_key, account_type, custodians, confirmations, custodians_public_keys, balance)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8::twa_account_type, $9, $10, $11, $12)
                RETURNING
                id, service_id as "service_id: _", workchain_id, hex, base64url, public_key, private_key, account_type as "account_type: _", custodians, confirmations, custodians_public_keys, balance, created_at, updated_at
"#,
                payload.id,
                payload.service_id as ServiceId,
                payload.workchain_id,
                payload.hex,
                payload.base64url,
                payload.public_key,
                payload.private_key,
                payload.account_type as AccountType,
                payload.custodians,
                payload.confirmations,
                payload.custodians_public_keys,
                balance
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn get_address(
        &self,
        service_id: ServiceId,
        workchain_id: i32,
        hex: String,
    ) -> Result<AddressDb> {
        sqlx::query_as!(AddressDb,
                r#"SELECT id, service_id as "service_id: _", workchain_id, hex, base64url, public_key, private_key, account_type as "account_type: _", custodians, confirmations, custodians_public_keys, balance, created_at, updated_at
                FROM address
                WHERE service_id = $1 AND workchain_id = $2 AND hex = $3"#,
                service_id as ServiceId,
                workchain_id,
                hex
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn get_address_by_workchain_hex(
        &self,
        workchain_id: i32,
        hex: String,
    ) -> Result<AddressDb> {
        sqlx::query_as!(AddressDb,
                r#"SELECT id, service_id as "service_id: _", workchain_id, hex, base64url, public_key, private_key, account_type as "account_type: _", custodians, confirmations, custodians_public_keys, balance, created_at, updated_at
                FROM address
                WHERE workchain_id = $1 AND hex = $2"#,
                workchain_id,
                hex
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn get_all_addresses(&self) -> Result<Vec<AddressDb>> {
        sqlx::query_as!(AddressDb,
                r#"SELECT id, service_id as "service_id: _", workchain_id, hex, base64url, public_key, private_key, account_type as "account_type: _", custodians, confirmations, custodians_public_keys, balance, created_at, updated_at
                FROM address"#
            )
            .fetch_all(&self.pool)
            .await
            .map_err(From::from)
    }
}
