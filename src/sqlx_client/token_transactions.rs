use anyhow::Result;

use crate::models::*;
use crate::prelude::*;
use crate::sqlx_client::*;

impl SqlxClient {
    pub async fn create_token_transaction(
        &self,
        payload: CreateTokenTransaction,
        service_id: ServiceId,
    ) -> Result<(TokenTransactionFromDb, TokenTransactionEventDb), ServiceError> {
        let mut tx = self.pool.begin().await.map_err(ServiceError::from)?;
        let transaction = sqlx::query_as!(TokenTransactionFromDb,
                r#"
            INSERT INTO token_transactions
            (id, service_id, transaction_hash, message_hash, owner_message_hash, account_workchain_id, account_hex, value, root_address, payload, error, block_hash, block_time, direction, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING id, service_id as "service_id: _", transaction_hash, message_hash, owner_message_hash, account_workchain_id, account_hex,
            value, root_address, payload, error, block_hash, block_time, direction as "direction: _", status as "status: _", created_at, updated_at"#,
                payload.id,
                service_id as ServiceId,
                payload.transaction_hash,
                payload.message_hash,
                payload.owner_message_hash,
                payload.account_workchain_id,
                payload.account_hex,
                payload.value,
                payload.root_address,
                payload.payload,
                payload.error,
                payload.block_hash,
                payload.block_time,
                payload.direction as TonTransactionDirection,
                payload.status as TonTokenTransactionStatus,
            )
            .fetch_one(&mut tx)
            .await
            .map_err(ServiceError::from)?;

        let payload = CreateTokenTransactionEvent::new(transaction.clone());

        let event = sqlx::query_as!(TokenTransactionEventDb,
                r#"
            INSERT INTO token_transaction_events
            (id, service_id, token_transaction_id, message_hash, account_workchain_id, account_hex, value, root_address, transaction_direction, transaction_status, event_status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id,
                service_id as "service_id: _",
                token_transaction_id,
                message_hash,
                account_workchain_id,
                account_hex,
                value,
                root_address,
                transaction_direction as "transaction_direction: _",
                transaction_status as "transaction_status: _",
                event_status as "event_status: _",
                created_at, updated_at"#,
                payload.id,
                payload.service_id as ServiceId,
                payload.token_transaction_id,
                payload.message_hash,
                payload.account_workchain_id,
                payload.account_hex,
                payload.value,
                payload.root_address,
                payload.transaction_direction as TonTransactionDirection,
                payload.transaction_status as TonTokenTransactionStatus,
                payload.event_status as TonEventStatus
            )
            .fetch_one(&mut tx)
            .await
            .map_err(ServiceError::from)?;

        tx.commit().await.map_err(ServiceError::from)?;

        Ok((transaction, event))
    }

    pub async fn get_token_transaction_by_mh(
        &self,
        service_id: ServiceId,
        message_hash: &str,
    ) -> Result<TokenTransactionFromDb, ServiceError> {
        sqlx::query_as!(TokenTransactionFromDb,
                r#"
            SELECT id, service_id as "service_id: _", transaction_hash, message_hash, owner_message_hash, account_workchain_id, account_hex,
            value, root_address, payload, error, block_hash, block_time, direction as "direction: _", status as "status: _", created_at, updated_at
            FROM token_transactions
            WHERE service_id = $1 AND message_hash = $2"#,
                service_id as ServiceId,
                message_hash,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn get_token_transaction_by_id(
        &self,
        service_id: ServiceId,
        id: &uuid::Uuid,
    ) -> Result<TokenTransactionFromDb, ServiceError> {
        sqlx::query_as!(TokenTransactionFromDb,
                r#"
            SELECT id, service_id as "service_id: _", transaction_hash, message_hash, owner_message_hash, account_workchain_id, account_hex,
            value, root_address, payload, error, block_hash, block_time, direction as "direction: _", status as "status: _", created_at, updated_at
            FROM token_transactions
            WHERE service_id = $1 AND id = $2"#,
                service_id as ServiceId,
                id,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn get_token_transaction_by_h(
        &self,
        service_id: ServiceId,
        transaction_hash: &str,
    ) -> Result<TokenTransactionFromDb, ServiceError> {
        sqlx::query_as!(TokenTransactionFromDb,
                r#"
            SELECT id, service_id as "service_id: _", transaction_hash, message_hash, owner_message_hash, account_workchain_id, account_hex,
            value, root_address, payload, error, block_hash, block_time, direction as "direction: _", status as "status: _", created_at, updated_at
            FROM token_transactions
            WHERE service_id = $1 AND transaction_hash = $2"#,
                service_id as ServiceId,
                transaction_hash,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }
}

#[cfg(test)]
mod test {}
