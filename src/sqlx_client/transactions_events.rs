use std::str::FromStr;

use anyhow::Result;
use itertools::Itertools;
use sqlx::postgres::PgArguments;
use sqlx::Arguments;
use sqlx::Row;

use crate::models::account_enums::TonEventStatus;
use crate::models::account_enums::TonTransactionDirection;
use crate::models::account_enums::TonTransactionStatus;
use crate::models::service_id::ServiceId;
use crate::models::sqlx::{TransactionDb, TransactionEventDb};
use crate::models::transaction_events::{
    CreateReceiveTransactionEvent, CreateSendTransactionEvent, UpdateSendTransactionEvent,
};
use crate::prelude::ServiceError;
use crate::sqlx_client::SqlxClient;

impl SqlxClient {
    pub async fn create_send_transaction_event(
        &self,
        payload: CreateSendTransactionEvent,
    ) -> Result<TransactionEventDb, ServiceError> {
        sqlx::query_as!(TransactionEventDb,
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
                balance_change,
                transaction_direction as "transaction_direction: _",
                transaction_status as "transaction_status: _",
                event_status as "event_status: _",
                created_at, updated_at"#,
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
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn update_send_transaction_event(
        &self,
        message_hash: String,
        payload: UpdateSendTransactionEvent,
    ) -> Result<TransactionEventDb, ServiceError> {
        sqlx::query_as!(
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
                balance_change,
                transaction_direction as "transaction_direction: _",
                transaction_status as "transaction_status: _",
                event_status as "event_status: _",
                created_at, updated_at"#,
            payload.balance_change,
            payload.transaction_status as TonTransactionStatus,
            message_hash,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }

    pub async fn create_receive_transaction_event(
        &self,
        payload: CreateReceiveTransactionEvent,
    ) -> Result<TransactionEventDb, ServiceError> {
        sqlx::query_as!(TransactionEventDb,
                r#"
            INSERT INTO transaction_events
            (id, service_id, transaction_id, message_hash, account_workchain_id, account_hex, balance_change, transaction_direction, transaction_status, event_status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id,
                service_id as "service_id: _",
                transaction_id,
                message_hash,
                account_workchain_id,
                account_hex,
                balance_change,
                transaction_direction as "transaction_direction: _",
                transaction_status as "transaction_status: _",
                event_status as "event_status: _",
                created_at, updated_at"#,
                payload.id,
                payload.service_id as ServiceId,
                payload.transaction_id,
                payload.message_hash,
                payload.account_workchain_id,
                payload.account_hex,
                payload.balance_change,
                payload.transaction_direction as TonTransactionDirection,
                payload.transaction_status as TonTransactionStatus,
                payload.event_status as TonEventStatus
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn get_transaction_event_by_mh(
        &self,
        message_hash: String,
        service_id: ServiceId,
        account_workchain_id: i32,
        account_hex: String,
    ) -> Result<TransactionEventDb, ServiceError> {
        sqlx::query_as!(
            TransactionEventDb,
            r#"
            SELECT id,
                service_id as "service_id: _",
                transaction_id,
                message_hash,
                account_workchain_id,
                account_hex,
                balance_change,
                transaction_direction as "transaction_direction: _",
                transaction_status as "transaction_status: _",
                event_status as "event_status: _",
                created_at, updated_at
            FROM transaction_events
            WHERE service_id = $1 AND message_hash = $2 AND account_workchain_id = $3 AND account_hex = $4"#,
            service_id as ServiceId,
            message_hash,
            account_workchain_id,
            account_hex,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }

    pub async fn update_event_status_of_transaction_event(
        &self,
        message_hash: String,
        account_workchain_id: i32,
        account_hex: String,
        event_status: TonEventStatus,
    ) -> Result<TransactionEventDb, ServiceError> {
        sqlx::query_as!(
            TransactionEventDb,
            r#"
            UPDATE transaction_events SET event_status = $1
            WHERE message_hash = $2 AND account_workchain_id = $3 AND account_hex = $4
            RETURNING id,
                service_id as "service_id: _",
                transaction_id,
                message_hash,
                account_workchain_id,
                account_hex,
                balance_change,
                transaction_direction as "transaction_direction: _",
                transaction_status as "transaction_status: _",
                event_status as "event_status: _",
                created_at, updated_at"#,
            event_status as TonEventStatus,
            message_hash,
            account_workchain_id,
            account_hex,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }
}

#[cfg(test)]
mod test {}
