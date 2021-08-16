use crate::models::account_enums::AccountType;
use crate::models::address::CreateAddressInDb;
use crate::models::service_id::ServiceId;
use crate::models::sqlx::AddressDb;
use crate::prelude::ServiceError;
use crate::sqlx_client::SqlxClient;

impl SqlxClient {
    pub async fn create_address(
        &self,
        payload: CreateAddressInDb,
    ) -> Result<AddressDb, ServiceError> {
        sqlx::query_as!(AddressDb,
                r#"INSERT INTO address
                (service_id, workchain_id, hex, base64url, public_key, private_key, account_type, custodians, confirmations, custodians_public_keys)
                VALUES ($1, $2, $3, $4, $5, $6, $7::twa_account_type, $8, $9, $10)
                RETURNING
                id, service_id as "service_id: _", workchain_id, hex, base64url, public_key, private_key, account_type as "account_type: _", custodians, confirmations, custodians_public_keys, balance, created_at, updated_at
"#,
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

    pub async fn get_address(
        &self,
        service_id: ServiceId,
        workchain_id: i32,
        hex: String,
    ) -> Result<AddressDb, ServiceError> {
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
}
