use anyhow::Result;
use chrono::prelude::*;

use crate::models::*;
use crate::sqlx_client::*;

impl SqlxClient {
    pub async fn create_token_transaction(
        &self,
        mut payload: CreateTokenTransaction,
        service_id: ServiceId,
    ) -> Result<(TokenTransactionFromDb, TokenTransactionEventDb)> {
        let mut tx = self.pool.begin().await?;

        if let Some(in_message_hash) = &payload.in_message_hash {
            let j_value = serde_json::json!(in_message_hash);
            if let Ok(transaction) = sqlx::query_as!(TransactionDb,
                r#"
            SELECT id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, transaction_timestamp, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, messages_hash, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, multisig_transaction_id, created_at, updated_at
            FROM transactions
            WHERE messages_hash @> $1::jsonb FOR UPDATE"#,
                j_value,
            )
                .fetch_one(&mut tx)
                .await {
                payload.owner_message_hash = Some(transaction.message_hash);
            }
        }

        let transaction_timestamp =
            NaiveDateTime::from_timestamp(payload.transaction_timestamp as i64, 0);

        let transaction = sqlx::query_as!(TokenTransactionFromDb,
                r#"
            INSERT INTO token_transactions
            (id, service_id, transaction_hash, transaction_timestamp, message_hash, owner_message_hash,
            account_workchain_id, account_hex, value, root_address, payload, error, block_hash, block_time,
            direction, status, in_message_hash)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            RETURNING id, service_id as "service_id: _", transaction_hash, transaction_timestamp, message_hash,
                owner_message_hash, account_workchain_id, account_hex, value, root_address, payload, error,
                block_hash, block_time, direction as "direction: _", status as "status: _", in_message_hash,
                created_at, updated_at"#,
                payload.id,
                service_id as ServiceId,
                payload.transaction_hash,
                transaction_timestamp,
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
                payload.in_message_hash,
            )
            .fetch_one(&mut tx)
            .await?;

        let payload = CreateTokenTransactionEvent::new(transaction.clone());

        let event = sqlx::query_as!(TokenTransactionEventDb,
                r#"
            INSERT INTO token_transaction_events
            (id, service_id, token_transaction_id, message_hash, account_workchain_id, account_hex,
            owner_message_hash,value, root_address, transaction_direction, transaction_status, event_status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING id,
                service_id as "service_id: _",
                token_transaction_id,
                message_hash,
                account_workchain_id,
                account_hex,
                owner_message_hash,
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
                payload.owner_message_hash,
                payload.value,
                payload.root_address,
                payload.transaction_direction as TonTransactionDirection,
                payload.transaction_status as TonTokenTransactionStatus,
                payload.event_status as TonEventStatus
            )
            .fetch_one(&mut tx)
            .await?;

        tx.commit().await?;

        Ok((transaction, event))
    }

    pub async fn get_token_transaction_by_mh(
        &self,
        service_id: ServiceId,
        message_hash: &str,
    ) -> Result<TokenTransactionFromDb> {
        sqlx::query_as!(TokenTransactionFromDb,
                r#"
            SELECT id, service_id as "service_id: _", transaction_hash, transaction_timestamp, message_hash, owner_message_hash, account_workchain_id, account_hex,
            value, root_address, payload, error, block_hash, block_time, direction as "direction: _", status as "status: _", in_message_hash, created_at, updated_at
            FROM token_transactions
            WHERE service_id = $1 AND (message_hash = $2 OR owner_message_hash = $2 OR in_message_hash = $2)"#,
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
    ) -> Result<TokenTransactionFromDb> {
        sqlx::query_as!(TokenTransactionFromDb,
                r#"
            SELECT id, service_id as "service_id: _", transaction_hash, transaction_timestamp, message_hash, owner_message_hash, account_workchain_id, account_hex,
            value, root_address, payload, error, block_hash, block_time, direction as "direction: _", status as "status: _", in_message_hash, created_at, updated_at
            FROM token_transactions
            WHERE service_id = $1 AND id = $2"#,
                service_id as ServiceId,
                id,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    #[allow(dead_code)]
    pub async fn get_token_transaction_by_h(
        &self,
        service_id: ServiceId,
        transaction_hash: &str,
    ) -> Result<TokenTransactionFromDb> {
        sqlx::query_as!(TokenTransactionFromDb,
                r#"
            SELECT id, service_id as "service_id: _", transaction_hash, transaction_timestamp, message_hash, owner_message_hash, account_workchain_id, account_hex,
            value, root_address, payload, error, block_hash, block_time, direction as "direction: _", status as "status: _", in_message_hash, created_at, updated_at
            FROM token_transactions
            WHERE service_id = $1 AND transaction_hash = $2"#,
                service_id as ServiceId,
                transaction_hash,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn update_token_transaction(
        &self,
        service_id: ServiceId,
        in_message_hash: &str,
        owner_message_hash: Option<String>,
    ) -> Result<Option<TokenTransactionEventDb>> {
        let mut tx = self.pool.begin().await?;

        let mut res = None;

        if let Some(token_transaction) = sqlx::query_as!(TokenTransactionFromDb,
                r#"
            SELECT id, service_id as "service_id: _", transaction_hash, transaction_timestamp, message_hash, owner_message_hash, account_workchain_id, account_hex,
            value, root_address, payload, error, block_hash, block_time, direction as "direction: _", status as "status: _", in_message_hash, created_at, updated_at
            FROM token_transactions
            WHERE service_id = $1 AND in_message_hash = $2"#,
                service_id as ServiceId,
                in_message_hash,
            )
            .fetch_optional(&mut tx)
            .await? {
            let updated_at = Utc::now().naive_utc();

            let _ = sqlx::query_as!(TokenTransactionFromDb,
            r#"
            UPDATE token_transactions SET (owner_message_hash, updated_at) = ($2, $3)
            WHERE id = $1
            RETURNING id, service_id as "service_id: _", transaction_hash, transaction_timestamp, message_hash,
                owner_message_hash, account_workchain_id, account_hex, value, root_address, payload, error,
                block_hash, block_time, direction as "direction: _", status as "status: _", in_message_hash,
                created_at, updated_at"#,
                token_transaction.id,
                owner_message_hash,
                updated_at
            )
                .fetch_one(&mut tx)
                .await?;

            let event = sqlx::query_as!(TokenTransactionEventDb,
            r#"
            UPDATE token_transaction_events SET (owner_message_hash, updated_at) = ($2, $3)
            WHERE token_transaction_id = $1
            RETURNING id,
                service_id as "service_id: _",
                token_transaction_id,
                message_hash,
                account_workchain_id,
                account_hex,
                owner_message_hash,
                value,
                root_address,
                transaction_direction as "transaction_direction: _",
                transaction_status as "transaction_status: _",
                event_status as "event_status: _",
                created_at, updated_at"#,
                token_transaction.id,
                owner_message_hash,
                updated_at
        )
                .fetch_one(&mut tx)
                .await?;

            res = Some(event);
        }

        tx.commit().await?;

        Ok(res)
    }
}

#[cfg(test)]
async fn prepare_test(level_filter: log::LevelFilter) -> SqlxClient {
    use env_logger::Builder;
    use sqlx::PgPool;
    use std::io::Write;
    use std::str::FromStr;

    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} {}/{} {} [{}] - {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.module_path().unwrap_or_default(),
                record.file().unwrap_or_default(),
                record.line().unwrap_or_default(),
                record.level(),
                record.args(),
            )
        })
        .filter(None, level_filter)
        .init();

    let pg_pool =
        PgPool::connect("postgresql://postgres:postgres@localhost:5432/ton_wallet_api_rs")
            .await
            .unwrap();

    let sqlx_client = SqlxClient::new(pg_pool);

    sqlx_client
}

#[cfg(test)]
mod test {
    use super::*;
    use log::LevelFilter;
    use std::str::FromStr;

    #[tokio::test]
    async fn test() {
        let sqlx_client = prepare_test(LevelFilter::Trace).await;

        let service_id =
            ServiceId::new(uuid::Uuid::from_str("5b30733f-e1cc-44e2-91f3-0ab7128e4534").unwrap());
        let message_hash = "8ec136e3833c1c2f490807668495209240c1a2ff1e22de253abada40a0ab81a7";
        let res = sqlx_client
            .get_token_transaction_by_mh(service_id, message_hash)
            .await
            .unwrap();
    }
}
