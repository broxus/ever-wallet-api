use anyhow::Result;
use itertools::Itertools;
use sqlx::postgres::PgArguments;
use sqlx::Arguments;
use sqlx::Row;
use uuid::Uuid;

use crate::models::*;
use crate::sqlx_client::*;

impl SqlxClient {
    #[allow(dead_code)]
    pub async fn get_transaction_event_by_mh(
        &self,
        service_id: ServiceId,
        message_hash: String,
        account_workchain_id: i32,
        account_hex: String,
    ) -> Result<TransactionEventDb> {
        sqlx::query_as!(
            TransactionEventDb,
            r#"
            SELECT te.id,
                te.service_id as "service_id: _",
                te.transaction_id,
                t.transaction_hash,
                te.message_hash,
                te.account_workchain_id,
                te.account_hex,
                te.sender_workchain_id,
                te.sender_hex,
                te.balance_change,
                te.transaction_direction as "transaction_direction: _",
                te.transaction_status as "transaction_status: _",
                te.event_status as "event_status: _",
                te.multisig_transaction_id, te.created_at, te.updated_at
            FROM transaction_events te
                LEFT JOIN transactions t on t.id = te.transaction_id
            WHERE te.service_id = $1 AND te.message_hash = $2 AND te.account_workchain_id = $3 AND te.account_hex = $4"#,
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
    ) -> Result<TransactionEventDb> {
        sqlx::query_as!(
            TransactionEventDb,
            r#"
            UPDATE transaction_events te SET event_status = $1
            FROM transactions t
            WHERE te.message_hash = $2 AND te.account_workchain_id = $3 AND te.account_hex = $4
                AND te.transaction_id = t.id
            RETURNING te.id,
                te.service_id as "service_id: _",
                te.transaction_id,
                t.transaction_hash,
                te.message_hash,
                te.account_workchain_id,
                te.account_hex,
                te.sender_workchain_id,
                te.sender_hex,
                te.balance_change,
                te.transaction_direction as "transaction_direction: _",
                te.transaction_status as "transaction_status: _",
                te.event_status as "event_status: _",
                te.multisig_transaction_id, te.created_at, te.updated_at"#,
            event_status as TonEventStatus,
            message_hash,
            account_workchain_id,
            account_hex,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }

    pub async fn update_event_status_of_transaction_event_by_id(
        &self,
        service_id: ServiceId,
        id: Uuid,
        event_status: TonEventStatus,
    ) -> Result<TransactionEventDb> {
        sqlx::query_as!(
            TransactionEventDb,
            r#"
            UPDATE transaction_events te SET event_status = $1
            FROM transactions t
            WHERE te.service_id = $2 AND te.id = $3
                AND te.transaction_id = t.id
            RETURNING te.id,
                te.service_id as "service_id: _",
                te.transaction_id,
                t.transaction_hash,
                te.message_hash,
                te.account_workchain_id,
                te.account_hex,
                te.sender_workchain_id,
                te.sender_hex,
                te.balance_change,
                te.transaction_direction as "transaction_direction: _",
                te.transaction_status as "transaction_status: _",
                te.event_status as "event_status: _",
                te.multisig_transaction_id, te.created_at, te.updated_at"#,
            event_status as TonEventStatus,
            service_id as ServiceId,
            id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }

    pub async fn update_event_status_of_transactions_event_by_status(
        &self,
        service_id: ServiceId,
        old_event_status: Option<TonEventStatus>,
        event_status: TonEventStatus,
    ) -> Result<Vec<TransactionEventDb>> {
        let mut args = PgArguments::default();
        args.add(event_status).expect("Failed to add query");
        args.add(service_id.inner()).expect("Failed to add query");

        let old = old_event_status
            .map(|old| {
                args.add(old).expect("Failed to add query");
                "AND te.event_status = $3"
            })
            .unwrap_or_default();
        let query = format!(
            r#"UPDATE transaction_events te SET event_status = $1
            FROM transactions t
            WHERE te.service_id = $2 {}
                AND te.transaction_id = t.id
            RETURNING id,
                te.service_id as "service_id: _",
                te.transaction_id,
                te.message_hash,
                te.account_workchain_id,
                te.account_hex,
                te.sender_workchain_id,
                te.sender_hex,
                te.balance_change,
                te.transaction_direction as "transaction_direction: _",
                te.transaction_status as "transaction_status: _",
                te.event_status as "event_status: _",
                te.multisig_transaction_id, te.created_at, te.updated_at,
                t.transaction_hash
            "#,
            old
        );

        let transactions = sqlx::query_with(&query, args).fetch_all(&self.pool).await?;

        let res = transactions
            .iter()
            .map(|x| TransactionEventDb {
                id: x.get(0),
                service_id: x.get(1),
                transaction_id: x.get(2),
                message_hash: x.get(3),
                account_workchain_id: x.get(4),
                account_hex: x.get(5),
                sender_workchain_id: x.get(6),
                sender_hex: x.get(7),
                balance_change: x.get(8),
                transaction_direction: x.get(9),
                transaction_status: x.get(10),
                event_status: x.get(11),
                multisig_transaction_id: x.get(12),
                created_at: x.get(13),
                updated_at: x.get(14),
                transaction_hash: x.get(15),
            })
            .collect::<Vec<_>>();
        Ok(res)
    }

    pub async fn get_event_by_id(
        &self,
        service_id: ServiceId,
        id: &Uuid,
    ) -> Result<TransactionEventDb> {
        sqlx::query_as!(
            TransactionEventDb,
            r#"
            SELECT te.id,
                te.service_id as "service_id: _",
                te.transaction_id,
                t.transaction_hash,
                te.message_hash,
                te.account_workchain_id,
                te.account_hex,
                te.sender_workchain_id,
                te.sender_hex,
                te.balance_change,
                te.transaction_direction as "transaction_direction: _",
                te.transaction_status as "transaction_status: _",
                te.event_status as "event_status: _",
                te.multisig_transaction_id,
                te.created_at,
                te.updated_at
            FROM transaction_events te
                LEFT JOIN transactions t ON t.id = te.transaction_id
            WHERE te.service_id = $1 AND te.id = $2"#,
            service_id as ServiceId,
            id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }

    pub async fn get_all_transaction_events(
        &self,
        service_id: ServiceId,
        input: &TransactionsEventsSearch,
    ) -> Result<Vec<TransactionEventDb>> {
        let mut args = PgArguments::default();
        args.add(service_id.inner()).expect("Failed to add query");

        let mut args_len = 1;

        let updates = filter_transaction_query(&mut args, &mut args_len, input);

        let query: String = format!(
            r#"SELECT
                te.id,
                te.service_id as "service_id: _",
                te.transaction_id,
                te.message_hash,
                te.account_workchain_id,
                te.account_hex,
                te.sender_workchain_id,
                te.sender_hex,
                te.balance_change,
                te.transaction_direction as "transaction_direction: _",
                te.transaction_status as "transaction_status: _",
                te.event_status as "event_status: _",
                te.multisig_transaction_id,
                te.created_at,
                te.updated_at,
                t.transaction_hash
                FROM transaction_events te
                    LEFT JOIN transactions t ON t.id = te.transaction_id
                WHERE te.service_id = $1 {} ORDER BY te.created_at DESC OFFSET ${} LIMIT ${}"#,
            updates.iter().format(""),
            args_len + 1,
            args_len + 2
        );

        args.add(input.offset).expect("Failed to add query");
        args.add(input.limit).expect("Failed to add query");

        let transactions = sqlx::query_with(&query, args).fetch_all(&self.pool).await?;

        let res = transactions
            .iter()
            .map(|x| TransactionEventDb {
                id: x.get(0),
                service_id: x.get(1),
                transaction_id: x.get(2),
                message_hash: x.get(3),
                account_workchain_id: x.get(4),
                account_hex: x.get(5),
                sender_workchain_id: x.get(6),
                sender_hex: x.get(7),
                balance_change: x.get(8),
                transaction_direction: x.get(9),
                transaction_status: x.get(10),
                event_status: x.get(11),
                multisig_transaction_id: x.get(12),
                created_at: x.get(13),
                updated_at: x.get(14),
                transaction_hash: x.get(15),
            })
            .collect::<Vec<_>>();
        Ok(res)
    }
}

pub fn filter_transaction_query(
    args: &mut PgArguments,
    args_len: &mut i32,
    input: &TransactionsEventsSearch,
) -> Vec<String> {
    let TransactionsEventsSearch {
        created_at_ge,
        created_at_le,
        transaction_id,
        message_hash,
        account_workchain_id,
        account_hex,
        transaction_direction,
        transaction_status,
        event_status,
        ..
    } = input.clone();
    let mut updates = Vec::new();

    if let Some(transaction_id) = transaction_id {
        updates.push(format!(" AND te.transaction_id = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(transaction_id).expect("Failed to add query")
    }

    if let Some(message_hash) = message_hash {
        updates.push(format!(" AND te.message_hash = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(message_hash).expect("Failed to add query")
    }

    if let Some(account_workchain_id) = account_workchain_id {
        updates.push(format!(
            " AND te.account_workchain_id = ${} ",
            *args_len + 1,
        ));
        *args_len += 1;
        args.add(account_workchain_id).expect("Failed to add query")
    }

    if let Some(account_hex) = account_hex {
        updates.push(format!(" AND te.account_hex = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(account_hex).expect("Failed to add query")
    }

    if let Some(transaction_direction) = transaction_direction {
        updates.push(format!(
            " AND te.transaction_direction = ${} ",
            *args_len + 1,
        ));
        *args_len += 1;
        args.add(transaction_direction)
            .expect("Failed to add query")
    }

    if let Some(transaction_status) = transaction_status {
        updates.push(format!(" AND te.transaction_status = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(transaction_status).expect("Failed to add query")
    }

    if let Some(event_status) = event_status {
        updates.push(format!(" AND te.event_status = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(event_status).expect("Failed to add query")
    }

    if let Some(created_at_ge) = created_at_ge {
        updates.push(format!(" AND te.created_at >= ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(created_at_ge).expect("Failed to add query")
    }

    if let Some(created_at_le) = created_at_le {
        updates.push(format!(" AND te.created_at <= ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(created_at_le).expect("Failed to add query")
    }

    updates
}

#[cfg(test)]
mod test {}
