use chrono::prelude::*;
use uuid::Uuid;

use crate::models::*;
use crate::prelude::*;
use crate::sqlx_client::*;

use itertools::Itertools;
use nekoton_utils::{repack_address, TrustMe};
use sqlx::postgres::PgArguments;
use sqlx::Arguments;
use sqlx::Row;

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
                transaction_scan_lt, transaction_timestamp, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, messages_hash, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at"#,
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
        let transaction_timestamp = payload.transaction_timestamp.map(|transaction_timestamp| {
            NaiveDateTime::from_timestamp(transaction_timestamp as i64, 0)
        });
        let updated_at = Utc::now().naive_utc();

        let transaction = sqlx::query_as!(TransactionDb,
                r#"
            UPDATE transactions SET
            (transaction_hash, transaction_lt, transaction_scan_lt, transaction_timestamp, sender_workchain_id, sender_hex, messages, messages_hash, data, value, fee, balance_change, status, error, updated_at) =
            ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            WHERE message_hash = $16 AND account_workchain_id = $17 and account_hex = $18 and direction = 'Send'::twa_transaction_direction and transaction_hash is NULL
            RETURNING id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, transaction_timestamp, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, messages_hash, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at"#,
                payload.transaction_hash,
                payload.transaction_lt,
                payload.transaction_scan_lt,
                transaction_timestamp,
                payload.sender_workchain_id,
                payload.sender_hex,
                payload.messages,
                payload.messages_hash,
                payload.data,
                payload.value,
                payload.fee,
                payload.balance_change,
                payload.status as TonTransactionStatus,
                payload.error,
                updated_at,
                message_hash,
                account_workchain_id,
                account_hex,
            )
            .fetch_one(&mut tx)
            .await
            .map_err(ServiceError::from)?;

        let payload = CreateSendTransactionEvent::new(transaction.clone());

        let event = sqlx::query_as!(TransactionEventDb,
                r#"
            INSERT INTO transaction_events
            (id, service_id, transaction_id, message_hash, account_workchain_id, account_hex, balance_change, transaction_direction, transaction_status, event_status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
        let transaction_timestamp =
            NaiveDateTime::from_timestamp(payload.transaction_timestamp.trust_me() as i64, 0);

        let transaction = sqlx::query_as!(TransactionDb,
                r#"
                 INSERT INTO transactions
            (id, service_id, message_hash, transaction_hash, transaction_lt, transaction_timestamp, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, messages_hash, data, value, fee, balance_change, direction, status, error, aborted, bounce)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
            RETURNING id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, transaction_timestamp, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, messages_hash, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at"#,
                transaction_id,
                service_id as ServiceId,
                message_hash,
                payload.transaction_hash,
                payload.transaction_lt,
                transaction_timestamp,
                payload.sender_workchain_id,
                payload.sender_hex,
                account_workchain_id,
                account_hex,
                payload.messages,
                payload.messages_hash,
                payload.data,
                payload.value,
                payload.fee,
                payload.balance_change,
                TonTransactionDirection::Send as TonTransactionDirection,
                payload.status as TonTransactionStatus,
                payload.error,
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
            (id, service_id, transaction_id, message_hash, account_workchain_id, account_hex, balance_change, transaction_direction, transaction_status, event_status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
                created_at, updated_at"#,
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
        let transaction_timestamp =
            NaiveDateTime::from_timestamp(payload.transaction_timestamp as i64, 0);

        let transaction = sqlx::query_as!(TransactionDb,
                r#"
            INSERT INTO transactions
            (id, service_id, message_hash, transaction_hash, transaction_lt, transaction_timeout, transaction_scan_lt, transaction_timestamp, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, messages_hash, data, original_value, original_outputs, value, fee, balance_change, direction, status, error, aborted, bounce)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25)
            RETURNING id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, transaction_timestamp, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, messages_hash, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at"#,
                payload.id,
                service_id as ServiceId,
                payload.message_hash,
                payload.transaction_hash,
                payload.transaction_lt,
                payload.transaction_timeout,
                payload.transaction_scan_lt,
                transaction_timestamp,
                payload.sender_workchain_id,
                payload.sender_hex,
                payload.account_workchain_id,
                payload.account_hex,
                payload.messages,
                payload.messages_hash,
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
            .fetch_one(&mut tx)
            .await
            .map_err(ServiceError::from)?;

        let payload = CreateReceiveTransactionEvent::new(transaction.clone());

        let event = sqlx::query_as!(TransactionEventDb,
                r#"
            INSERT INTO transaction_events
            (id, service_id, transaction_id, message_hash, account_workchain_id, account_hex, sender_workchain_id, sender_hex, balance_change, transaction_direction, transaction_status, event_status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
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
                created_at, updated_at"#,
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
                payload.event_status as TonEventStatus
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
                transaction_scan_lt, transaction_timestamp, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, messages_hash, data,
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
                transaction_scan_lt, transaction_timestamp, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, messages_hash, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at
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
                transaction_scan_lt, transaction_timestamp, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, messages_hash, data,
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
    pub async fn get_transaction_by_id(
        &self,
        service_id: ServiceId,
        id: &uuid::Uuid,
    ) -> Result<TransactionDb, ServiceError> {
        sqlx::query_as!(TransactionDb,
                r#"
            SELECT id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, transaction_timestamp, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, messages_hash, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at
            FROM transactions
            WHERE service_id = $1 AND id = $2"#,
                service_id as ServiceId,
                id,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    #[allow(dead_code)]
    pub async fn get_all_transactions_by_status(
        &self,
        status: TonTransactionStatus,
    ) -> Result<Vec<TransactionDb>, ServiceError> {
        sqlx::query_as!(TransactionDb,
                r#"
            SELECT id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, transaction_timestamp, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, messages_hash, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at
            FROM transactions
            WHERE status = $1"#,
                status as TonTransactionStatus,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn get_transaction_by_out_msg(
        &self,
        message_hash: String,
    ) -> Result<TransactionDb, ServiceError> {
        let j_value = serde_json::json!(message_hash);
        sqlx::query_as!(TransactionDb,
                r#"
            SELECT id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, transaction_timestamp, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, messages_hash, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at
            FROM transactions
            WHERE messages_hash @> $1::jsonb"#,
                j_value,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(From::from)
    }

    pub async fn get_all_transactions(
        &self,
        service_id: ServiceId,
        input: &TransactionsSearch,
    ) -> Result<Vec<TransactionDb>, ServiceError> {
        let mut args = PgArguments::default();
        args.add(service_id.inner());
        let mut args_len = 1;

        let order_by = match input.ordering {
            None => "",
            Some(ref o) => match o {
                TransactionsSearchOrdering::CreatedAtAsc => "ORDER BY created_at asc",
                TransactionsSearchOrdering::CreatedAtDesc => "ORDER BY created_at desc",
                TransactionsSearchOrdering::TransactionLtAsc => "ORDER BY transaction_lt asc",
                TransactionsSearchOrdering::TransactionLtDesc => "ORDER BY transaction_lt desc",
                TransactionsSearchOrdering::TransactionTimestampAsc => {
                    "ORDER BY transaction_timestamp asc"
                }
                TransactionsSearchOrdering::TransactionTimestampDesc => {
                    "ORDER BY transaction_timestamp desc"
                }
            },
        };

        let updates = filter_transaction_query(&mut args, &mut args_len, input);

        let query: String = format!(
            r#"SELECT id, service_id as "service_id: _", message_hash, transaction_hash, transaction_lt, transaction_timeout,
                transaction_scan_lt, transaction_timestamp, sender_workchain_id, sender_hex, account_workchain_id, account_hex, messages, messages_hash, data,
                original_value, original_outputs, value, fee, balance_change, direction as "direction: _", status as "status: _",
                error, aborted, bounce, created_at, updated_at
                FROM transactions WHERE service_id = $1 {updates} {order_by} OFFSET ${offset} LIMIT ${limit}"#,
            updates = updates.iter().format(""),
            order_by = order_by,
            offset = args_len + 1,
            limit = args_len + 2
        );

        args.add(input.offset);
        args.add(input.limit);
        let transactions = sqlx::query_with(&query, args).fetch_all(&self.pool).await?;

        let res = transactions
            .iter()
            .map(|x| TransactionDb {
                id: x.get(0),
                service_id: x.get(1),
                message_hash: x.get(2),
                transaction_hash: x.get(3),
                transaction_lt: x.get(4),
                transaction_timeout: x.get(5),
                transaction_scan_lt: x.get(6),
                transaction_timestamp: x.get(7),
                sender_workchain_id: x.get(8),
                sender_hex: x.get(9),
                account_workchain_id: x.get(10),
                account_hex: x.get(11),
                messages: x.get(12),
                messages_hash: x.get(13),
                data: x.get(14),
                original_value: x.get(15),
                original_outputs: x.get(16),
                value: x.get(17),
                fee: x.get(18),
                balance_change: x.get(19),
                direction: x.get(20),
                status: x.get(21),
                error: x.get(22),
                aborted: x.get(23),
                bounce: x.get(24),
                created_at: x.get(25),
                updated_at: x.get(26),
            })
            .collect::<Vec<_>>();
        Ok(res)
    }
}

pub fn filter_transaction_query(
    args: &mut PgArguments,
    args_len: &mut i32,
    input: &TransactionsSearch,
) -> Vec<String> {
    let TransactionsSearch {
        id,
        message_hash,
        transaction_hash,
        account,
        status,
        direction,
        created_at_min,
        created_at_max,
        ..
    } = input.clone();
    let mut updates = Vec::new();

    if let Some(id) = id {
        updates.push(format!(" AND id = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(id)
    }

    if let Some(transaction_hash) = transaction_hash {
        updates.push(format!(" AND transaction_hash = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(transaction_hash)
    }

    if let Some(message_hash) = message_hash {
        updates.push(format!(" AND message_hash = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(message_hash)
    }

    if let Some(account) = account {
        if let Ok(account) = repack_address(&account) {
            updates.push(format!(" AND account_workchain_id = ${} ", *args_len + 1,));
            *args_len += 1;
            args.add(account.workchain_id());
            updates.push(format!(" AND account_hex = ${} ", *args_len + 1,));
            *args_len += 1;
            args.add(account.address().to_hex_string());
        }
    }

    if let Some(status) = status {
        updates.push(format!(" AND status = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(status)
    }

    if let Some(direction) = direction {
        updates.push(format!(" AND direction = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(direction)
    }

    if let Some(created_at_min) = created_at_min {
        updates.push(format!(" AND created_at >= ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(NaiveDateTime::from_timestamp(
            created_at_min / 1000,
            ((created_at_min % 1000) * 1_000_000) as u32,
        ))
    }

    if let Some(created_at_max) = created_at_max {
        updates.push(format!(" AND created_at <= ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(NaiveDateTime::from_timestamp(
            created_at_max / 1000,
            ((created_at_max % 1000) * 1_000_000) as u32,
        ))
    }

    updates
}

#[cfg(test)]
mod test {}
