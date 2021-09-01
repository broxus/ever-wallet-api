use itertools::Itertools;
use sqlx::postgres::PgArguments;
use sqlx::Arguments;
use sqlx::Row;
use uuid::Uuid;

use crate::models::*;
use crate::prelude::*;
use crate::sqlx_client::*;

impl SqlxClient {
    pub async fn get_token_transaction_event_by_mh(
        &self,
        message_hash: String,
        service_id: ServiceId,
        account_workchain_id: i32,
        account_hex: String,
    ) -> Result<TokenTransactionEventDb, ServiceError> {
        sqlx::query_as!(
            TokenTransactionEventDb,
            r#"
            SELECT id,
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
                created_at, updated_at
            FROM token_transaction_events
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

    pub async fn update_event_status_of_token_transaction_event(
        &self,
        message_hash: String,
        account_workchain_id: i32,
        account_hex: String,
        event_status: TonEventStatus,
    ) -> Result<TokenTransactionEventDb, ServiceError> {
        sqlx::query_as!(
            TokenTransactionEventDb,
            r#"
            UPDATE token_transaction_events SET event_status = $1
            WHERE message_hash = $2 AND account_workchain_id = $3 AND account_hex = $4
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
            event_status as TonEventStatus,
            message_hash,
            account_workchain_id,
            account_hex,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }

    pub async fn get_token_transaction_events(
        &self,
        service_id: ServiceId,
        event_status: TonEventStatus,
    ) -> Result<Vec<TokenTransactionEventDb>, ServiceError> {
        sqlx::query_as!(
            TokenTransactionEventDb,
            r#"
            SELECT id,
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
                created_at, updated_at
            FROM token_transaction_events
            WHERE service_id = $1 AND event_status = $2"#,
            service_id as ServiceId,
            event_status as TonEventStatus,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(From::from)
    }

    pub async fn update_event_status_of_token_transaction_event_by_id(
        &self,
        service_id: ServiceId,
        id: Uuid,
        event_status: TonEventStatus,
    ) -> Result<TokenTransactionEventDb, ServiceError> {
        sqlx::query_as!(
            TokenTransactionEventDb,
            r#"
            UPDATE token_transaction_events SET event_status = $1
            WHERE service_id = $2 AND id = $3
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
            event_status as TonEventStatus,
            service_id as ServiceId,
            id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }

    pub async fn get_all_token_transaction_events(
        &self,
        service_id: ServiceId,
        input: &TokenTransactionsEventsSearch,
    ) -> Result<Vec<TokenTransactionEventDb>, ServiceError> {
        let mut args = PgArguments::default();
        args.add(service_id.inner());
        let mut args_len = 1;

        let updates = filter_token_transaction_query(&mut args, &mut args_len, input);

        let query: String = format!(
            r#"SELECT
                id,
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
                created_at,
                updated_at
                FROM token_transaction_events WHERE service_id = $1 {} ORDER BY created_at DESC OFFSET ${} LIMIT ${}"#,
            updates.iter().format(""),
            args_len + 1,
            args_len + 2
        );

        args.add(input.offset);
        args.add(input.limit);
        let transactions = sqlx::query_with(&query, args).fetch_all(&self.pool).await?;

        let res = transactions
            .iter()
            .map(|x| TokenTransactionEventDb {
                id: x.get(0),
                service_id: x.get(1),
                token_transaction_id: x.get(2),
                message_hash: x.get(3),
                account_workchain_id: x.get(4),
                account_hex: x.get(5),
                value: x.get(6),
                root_address: x.get(7),
                transaction_direction: x.get(8),
                transaction_status: x.get(9),
                event_status: x.get(10),
                created_at: x.get(11),
                updated_at: x.get(12),
            })
            .collect::<Vec<_>>();
        Ok(res)
    }
}

pub fn filter_token_transaction_query(
    args: &mut PgArguments,
    args_len: &mut i32,
    input: &TokenTransactionsEventsSearch,
) -> Vec<String> {
    let TokenTransactionsEventsSearch {
        created_at_ge,
        created_at_le,
        token_transaction_id,
        root_address,
        message_hash,
        account_workchain_id,
        account_hex,
        transaction_direction,
        transaction_status,
        event_status,
        ..
    } = input.clone();
    let mut updates = Vec::new();

    if let Some(token_transaction_id) = token_transaction_id {
        updates.push(format!(" AND token_transaction_id = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(token_transaction_id)
    }

    if let Some(root_address) = root_address {
        updates.push(format!(" AND root_address = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(root_address)
    }

    if let Some(message_hash) = message_hash {
        updates.push(format!(" AND message_hash = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(message_hash)
    }

    if let Some(account_workchain_id) = account_workchain_id {
        updates.push(format!(" AND account_workchain_id = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(account_workchain_id)
    }

    if let Some(account_hex) = account_hex {
        updates.push(format!(" AND account_hex = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(account_hex)
    }

    if let Some(transaction_direction) = transaction_direction {
        updates.push(format!(" AND transaction_direction = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(transaction_direction)
    }

    if let Some(transaction_status) = transaction_status {
        updates.push(format!(" AND transaction_status = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(transaction_status)
    }

    if let Some(event_status) = event_status {
        updates.push(format!(" AND event_status = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(event_status)
    }

    if let Some(created_at_ge) = created_at_ge {
        updates.push(format!(" AND created_at >= ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(created_at_ge)
    }

    if let Some(created_at_le) = created_at_le {
        updates.push(format!(" AND created_at <= ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(created_at_le)
    }

    updates
}

#[cfg(test)]
mod test {}
