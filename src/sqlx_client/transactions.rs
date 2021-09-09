use uuid::Uuid;

use crate::models::*;
use crate::prelude::*;
use crate::sqlx_client::*;

impl SqlxClient {
    pub async fn create_send_transaction(
        &self,
        payload: CreateSendTransaction,
    ) -> Result<(TransactionDb, TransactionEventDb), ServiceError> {
        let mut tx = self.pool.begin().await.map_err(ServiceError::from)?;
        let transaction = sqlx::query_as!(TransactionDb,
                r#"
            INSERT INTO transactions
            (id, service_id, message_hash, account_workchain_id, account_hex, original_value, original_outputs, direction, status, aborted, bounce)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at, sender_is_token_wallet"#,
                payload.id,
                payload.service_id as ServiceId,
                payload.message_hash,
                payload.account_workchain_id,
                payload.account_hex,
                payload.original_value,
                payload.original_outputs,
                payload.direction as TonTransactionDirection,
                payload.status as TonTransactionStatus,
                payload.aborted,
                payload.bounce,
            )
            .fetch_one(&mut tx)
            .await
            .map_err(ServiceError::from)?;

        let payload = CreateSendTransactionEvent::new(transaction.clone());

        let event = sqlx::query_as!(TransactionEventDb,
                r#"
            INSERT INTO transaction_events
            (id, service_id, transaction_id, message_hash, account_workchain_id, account_hex, transaction_direction, transaction_status, event_status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                id,
                service_id as "service_id: _",
                transaction_id,
                message_hash,
                account_workchain_id,
                account_hex,
                sender_workchain_id,
                sender_hex,
                balance_change,
                transaction_direction as "transaction_direction: _",
                transaction_status as "transaction_status: _",
                event_status as "event_status: _",
                created_at, updated_at, sender_is_token_wallet"#,
                payload.id,
                payload.service_id as ServiceId,
                payload.transaction_id,
                payload.message_hash,
                payload.account_workchain_id,
                payload.account_hex,
                payload.transaction_direction as TonTransactionDirection,
                payload.transaction_status as TonTransactionStatus,
                payload.event_status as TonEventStatus
            )
            .fetch_one(&mut tx)
            .await
            .map_err(ServiceError::from)?;

        tx.commit().await.map_err(ServiceError::from)?;

        Ok((transaction, event))
    }

    pub async fn update_send_transaction(
        &self,
        message_hash: String,
        account_workchain_id: i32,
        account_hex: String,
        payload: UpdateSendTransaction,
    ) -> Result<(TransactionDb, TransactionEventDb), ServiceError> {
        let mut tx = self.pool.begin().await.map_err(ServiceError::from)?;
        let transaction = sqlx::query_as!(TransactionDb,
                r#"
            UPDATE transactions SET
            (transaction_hash, transaction_lt, transaction_scan_lt, sender_workchain_id, sender_hex, messages, data, value, fee, balance_change, status, error) =
            ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            WHERE message_hash = $13 AND account_workchain_id = $14 and account_hex = $15 and direction = 'Send'::twa_transaction_direction and transaction_hash is NULL
            RETURNING id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at, sender_is_token_wallet"#,
                payload.transaction_hash,
                payload.transaction_lt,
                payload.transaction_scan_lt,
                payload.sender_workchain_id,
                payload.sender_hex,
                payload.messages,
                payload.data,
                payload.value,
                payload.fee,
                payload.balance_change,
                payload.status as TonTransactionStatus,
                payload.error,
                message_hash,
                account_workchain_id,
                account_hex,
            )
            .fetch_one(&mut tx)
            .await
            .map_err(ServiceError::from)?;

        let payload = UpdateSendTransactionEvent::new(transaction.clone());

        let event = sqlx::query_as!(
            TransactionEventDb,
            r#"
            UPDATE transaction_events SET
            (balance_change, transaction_status) =
            ($1, $2)
            WHERE message_hash = $3 and transaction_direction = 'Send'::twa_transaction_direction
            RETURNING id,
                service_id as "service_id: _",
                transaction_id,
                message_hash,
                account_workchain_id,
                account_hex,
                sender_workchain_id,
                sender_hex,
                balance_change,
                transaction_direction as "transaction_direction: _",
                transaction_status as "transaction_status: _",
                event_status as "event_status: _",
                created_at, updated_at, sender_is_token_wallet"#,
            payload.balance_change,
            payload.transaction_status as TonTransactionStatus,
            message_hash,
        )
        .fetch_one(&mut tx)
        .await
        .map_err(ServiceError::from)?;

        tx.commit().await.map_err(ServiceError::from)?;

        Ok((transaction, event))
    }

    pub async fn create_sent_transaction(
        &self,
        service_id: ServiceId,
        message_hash: String,
        account_workchain_id: i32,
        account_hex: String,
        payload: UpdateSendTransaction,
    ) -> Result<(TransactionDb, TransactionEventDb), ServiceError> {
        let mut tx = self.pool.begin().await.map_err(ServiceError::from)?;
        let transaction_id = Uuid::new_v4();
        let transaction = sqlx::query_as!(TransactionDb,
                r#"
                 INSERT INTO transactions
            (id, service_id, message_hash, transaction_hash, transaction_lt, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, data, value, fee, balance_change, direction, status, error, aborted, bounce, sender_is_token_wallet)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
            RETURNING id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at, sender_is_token_wallet"#,
                transaction_id,
                service_id as ServiceId,
                message_hash,
                payload.transaction_hash,
                payload.transaction_lt,
                payload.sender_workchain_id,
                payload.sender_hex,
                account_workchain_id,
                account_hex,
                payload.messages,
                payload.data,
                payload.value,
                payload.fee,
                payload.balance_change,
                TonTransactionDirection::Send as TonTransactionDirection,
                payload.status as TonTransactionStatus,
                payload.error,
                false,
                false,
                false
            )
            .fetch_one(&mut tx)
            .await
            .map_err(ServiceError::from)?;

        let payload = UpdateSendTransactionEvent::new(transaction.clone());
        let id = Uuid::new_v4();

        let event = sqlx::query_as!(
            TransactionEventDb,
            r#"
            INSERT INTO transaction_events
            (id, service_id, transaction_id, message_hash, account_workchain_id, account_hex, balance_change, transaction_direction, transaction_status, event_status, sender_is_token_wallet)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id,
                service_id as "service_id: _",
                transaction_id,
                message_hash,
                account_workchain_id,
                account_hex,
                sender_workchain_id,
                sender_hex,
                balance_change,
                transaction_direction as "transaction_direction: _",
                transaction_status as "transaction_status: _",
                event_status as "event_status: _",
                created_at, updated_at, sender_is_token_wallet"#,
                id,
                service_id as ServiceId,
                transaction_id,
                message_hash,
                account_workchain_id,
                account_hex,
                payload.balance_change,
                TonTransactionDirection::Send as TonTransactionDirection,
                payload.transaction_status as TonTransactionStatus,
                TonEventStatus::New as TonEventStatus,
                false,
        )
        .fetch_one(&mut tx)
        .await
        .map_err(ServiceError::from)?;

        tx.commit().await.map_err(ServiceError::from)?;

        Ok((transaction, event))
    }

    pub async fn create_receive_transaction(
        &self,
        payload: CreateReceiveTransaction,
        service_id: ServiceId,
    ) -> Result<(TransactionDb, TransactionEventDb), ServiceError> {
        let mut tx = self.pool.begin().await.map_err(ServiceError::from)?;

        let transaction = sqlx::query_as!(TransactionDb,
                r#"
            INSERT INTO transactions
            (id, service_id, message_hash, transaction_hash, transaction_lt, transaction_timeout, transaction_scan_lt, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, data, original_value, original_outputs, value, fee, balance_change, direction, status, error, aborted, bounce, sender_is_token_wallet)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
            RETURNING id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at, sender_is_token_wallet"#,
                payload.id,
                service_id as ServiceId,
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
                payload.bounce,
                payload.sender_is_token_wallet
            )
            .fetch_one(&mut tx)
            .await
            .map_err(ServiceError::from)?;

        let payload = CreateReceiveTransactionEvent::new(transaction.clone());

        let event = sqlx::query_as!(TransactionEventDb,
                r#"
            INSERT INTO transaction_events
            (id, service_id, transaction_id, message_hash, account_workchain_id, account_hex, sender_workchain_id, sender_hex, balance_change, transaction_direction, transaction_status, event_status, sender_is_token_wallet)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING id,
                service_id as "service_id: _",
                transaction_id,
                message_hash,
                account_workchain_id,
                account_hex,
                sender_workchain_id,
                sender_hex,
                balance_change,
                transaction_direction as "transaction_direction: _",
                transaction_status as "transaction_status: _",
                event_status as "event_status: _",
                created_at, updated_at, sender_is_token_wallet"#,
                payload.id,
                payload.service_id as ServiceId,
                payload.transaction_id,
                payload.message_hash,
                payload.account_workchain_id,
                payload.account_hex,
                payload.sender_workchain_id,
                payload.sender_hex,
                payload.balance_change,
                payload.transaction_direction as TonTransactionDirection,
                payload.transaction_status as TonTransactionStatus,
                payload.event_status as TonEventStatus,
                payload.sender_is_token_wallet,
            )
            .fetch_one(&mut tx)
            .await
            .map_err(ServiceError::from)?;

        tx.commit().await.map_err(ServiceError::from)?;

        Ok((transaction, event))
    }

    pub async fn get_transaction_by_mh(
        &self,
        service_id: ServiceId,
        message_hash: &str,
    ) -> Result<TransactionDb, ServiceError> {
        sqlx::query_as!(TransactionDb,
                r#"
            SELECT id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at, sender_is_token_wallet
            FROM transactions
            WHERE service_id = $1 AND message_hash = $2"#,
                service_id as ServiceId,
                message_hash,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn get_sent_transaction_by_mh_account(
        &self,
        service_id: ServiceId,
        message_hash: String,
        account_workchain_id: i32,
        account_hex: String,
    ) -> Result<Option<TransactionDb>, ServiceError> {
        sqlx::query_as!(TransactionDb,
                r#"
            SELECT id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at, sender_is_token_wallet
            FROM transactions
            WHERE service_id = $1 AND message_hash = $2 AND account_workchain_id = $3 AND account_hex = $4 and direction = 'Send'::twa_transaction_direction"#,
                service_id as ServiceId,
                message_hash,
                account_workchain_id,
                account_hex,
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn get_transaction_by_h(
        &self,
        service_id: ServiceId,
        transaction_hash: &str,
    ) -> Result<TransactionDb, ServiceError> {
        sqlx::query_as!(TransactionDb,
                r#"
            SELECT id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at, sender_is_token_wallet
            FROM transactions
            WHERE service_id = $1 AND transaction_hash = $2"#,
                service_id as ServiceId,
                transaction_hash,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }
    pub async fn get_transaction_by_id(
        &self,
        service_id: ServiceId,
        id: &uuid::Uuid,
    ) -> Result<TransactionDb, ServiceError> {
        sqlx::query_as!(TransactionDb,
                r#"
            SELECT id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at, sender_is_token_wallet
            FROM transactions
            WHERE service_id = $1 AND id = $2"#,
                service_id as ServiceId,
                id,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }
}

#[cfg(test)]
mod test {}
