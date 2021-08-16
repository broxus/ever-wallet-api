use anyhow::Result;

use crate::models::account_enums::TonTransactionDirection;
use crate::models::account_enums::TonTransactionStatus;
use crate::models::service_id::ServiceId;
use crate::models::sqlx::TransactionDb;
use crate::models::transactions::{
    CreateReceiveTransaction, CreateSendTransaction, UpdateSendTransaction,
};
use crate::prelude::ServiceError;
use crate::sqlx_client::SqlxClient;

impl SqlxClient {
    pub async fn create_send_transaction(
        &self,
        payload: CreateSendTransaction,
    ) -> Result<TransactionDb, ServiceError> {
        sqlx::query_as!(TransactionDb,
                r#"
            INSERT INTO transactions
            (id, service_id, message_hash, account_workchain_id, account_hex, value, direction, status, aborted, bounce)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at"#,
                payload.id,
                payload.service_id as ServiceId,
                payload.message_hash,
                payload.account_workchain_id,
                payload.account_hex,
                payload.value,
                payload.direction as TonTransactionDirection,
                payload.status as TonTransactionStatus,
                payload.aborted,
                payload.bounce,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn update_send_transaction(
        &self,
        message_hash: String,
        account_workchain_id: i32,
        account_hex: String,
        payload: UpdateSendTransaction,
    ) -> Result<TransactionDb, ServiceError> {
        sqlx::query_as!(TransactionDb,
                r#"
            UPDATE transactions SET
            (transaction_hash, transaction_lt, transaction_timeout, transaction_scan_lt, sender_workchain_id, sender_hex, messages, data, original_value, original_outputs, value, fee, balance_change, status, error) =
            ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            WHERE message_hash = $16 AND account_workchain_id = $17 and account_hex = $18 and direction = 'Send'::twa_transaction_direction and transaction_hash = NULL
            RETURNING id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at"#,
                payload.transaction_hash,
                payload.transaction_lt,
                payload.transaction_timeout,
                payload.transaction_scan_lt,
                payload.sender_workchain_id,
                payload.sender_hex,
                payload.messages,
                payload.data,
                payload.original_value,
                payload.original_outputs,
                payload.value,
                payload.fee,
                payload.balance_change,
                payload.status as TonTransactionStatus,
                payload.error,
                message_hash,
                account_workchain_id,
                account_hex,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn create_receive_transaction(
        &self,
        payload: CreateReceiveTransaction,
    ) -> Result<TransactionDb, ServiceError> {
        sqlx::query_as!(TransactionDb,
                r#"
            INSERT INTO transactions
            (id, service_id, message_hash, transaction_hash, transaction_lt, transaction_timeout, transaction_scan_lt, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, data, original_value, original_outputs, value, fee, balance_change, direction, status, error, aborted, bounce)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
            RETURNING id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at"#,
                payload.id,
                payload.service_id as ServiceId,
                payload.message_hash,
                payload.transaction_hash,
                payload.transaction_lt,
                payload.transaction_timeout,
                payload.transaction_scan_lt,
                payload.sender_workchain_id,
                payload.sender_hex,
                payload.account_workchain_id,
                payload.account_hex,
                payload.messages,
                payload.data,
                payload.original_value,
                payload.original_outputs,
                payload.value,
                payload.fee,
                payload.balance_change,
                payload.direction as TonTransactionDirection,
                payload.status as TonTransactionStatus,
                payload.error,
                payload.aborted,
                payload.bounce
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn get_transaction_by_mh(
        &self,
        message_hash: String,
        service_id: ServiceId,
    ) -> Result<TransactionDb, ServiceError> {
        sqlx::query_as!(TransactionDb,
                r#"
            SELECT id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at
            FROM transactions
            WHERE service_id = $1 AND message_hash = $2"#,
                service_id as ServiceId,
                message_hash,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn get_transaction_by_h(
        &self,
        transaction_hash: String,
        service_id: ServiceId,
    ) -> Result<TransactionDb, ServiceError> {
        sqlx::query_as!(TransactionDb,
                r#"
            SELECT id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at
            FROM transactions
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
