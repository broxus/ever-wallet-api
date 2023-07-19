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
    pub async fn get_token_transaction_event_by_mh(
        &self,
        message_hash: String,
        service_id: ServiceId,
        account_workchain_id: i32,
        account_hex: String,
    ) -> Result<TokenTransactionEventDb> {
        sqlx::query_as!(
            TokenTransactionEventDb,
            r#"
            SELECT tte.id,
                tte.service_id as "service_id: _",
                tte.token_transaction_id,
                tt.transaction_hash as token_transaction_hash,
                tte.message_hash,
                tte.account_workchain_id,
                tte.account_hex,
                tte.owner_message_hash,
                tte.value,
                tte.root_address,
                tte.transaction_direction as "transaction_direction: _",
                tte.transaction_status as "transaction_status: _",
                tte.event_status as "event_status: _",
                tte.created_at, tte.updated_at
            FROM token_transaction_events tte
                LEFT JOIN token_transactions tt on tt.id = tte.token_transaction_id
            WHERE tte.service_id = $1 AND tte.message_hash = $2 AND tte.account_workchain_id = $3 AND tte.account_hex = $4"#,
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
    ) -> Result<TokenTransactionEventDb> {
        sqlx::query_as!(
            TokenTransactionEventDb,
            r#"
            UPDATE token_transaction_events tte SET event_status = $1
            FROM token_transactions tt
            WHERE tte.message_hash = $2 AND tte.account_workchain_id = $3 AND tte.account_hex = $4
                AND tte.token_transaction_id = tt.id
            RETURNING tte.id,
                tte.service_id as "service_id: _",
                tte.token_transaction_id,
                tt.transaction_hash as token_transaction_hash,
                tte.message_hash,
                tte.account_workchain_id,
                tte.account_hex,
                tte.owner_message_hash,
                tte.value,
                tte.root_address,
                tte.transaction_direction as "transaction_direction: _",
                tte.transaction_status as "transaction_status: _",
                tte.event_status as "event_status: _",
                tte.created_at, tte.updated_at"#,
            event_status as TonEventStatus,
            message_hash,
            account_workchain_id,
            account_hex,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }

    #[allow(dead_code)]
    pub async fn get_token_transaction_events(
        &self,
        service_id: ServiceId,
        event_status: TonEventStatus,
    ) -> Result<Vec<TokenTransactionEventDb>> {
        sqlx::query_as!(
            TokenTransactionEventDb,
            r#"
            SELECT tte.id,
                tte.service_id as "service_id: _",
                tte.token_transaction_id,
                tt.transaction_hash as token_transaction_hash,
                tte.message_hash,
                tte.account_workchain_id,
                tte.account_hex,
                tte.owner_message_hash,
                tte.value,
                tte.root_address,
                tte.transaction_direction as "transaction_direction: _",
                tte.transaction_status as "transaction_status: _",
                tte.event_status as "event_status: _",
                tte.created_at, tte.updated_at
            FROM token_transaction_events tte
                LEFT JOIN token_transactions tt on tt.id = tte.token_transaction_id
            WHERE tte.service_id = $1 AND tte.event_status = $2"#,
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
    ) -> Result<TokenTransactionEventDb> {
        sqlx::query_as!(
            TokenTransactionEventDb,
            r#"
            UPDATE token_transaction_events tte SET event_status = $1
            FROM token_transactions tt
            WHERE tte.service_id = $2 AND tte.id = $3
                AND tte.token_transaction_id = tt.id
            RETURNING tte.id,
                tte.service_id as "service_id: _",
                tte.token_transaction_id,
                tt.transaction_hash as token_transaction_hash,
                tte.message_hash,
                tte.account_workchain_id,
                tte.account_hex,
                tte.owner_message_hash,
                tte.value,
                tte.root_address,
                tte.transaction_direction as "transaction_direction: _",
                tte.transaction_status as "transaction_status: _",
                tte.event_status as "event_status: _",
                tte.created_at, tte.updated_at"#,
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
    ) -> Result<Vec<TokenTransactionEventDb>> {
        let mut args = PgArguments::default();
        args.add(service_id.inner());
        let mut args_len = 1;

        let updates = filter_token_transaction_query(&mut args, &mut args_len, input);

        let query: String = format!(
            r#"SELECT
                tte.id,
                tte.service_id as "service_id: _",
                tte.token_transaction_id,
                tte.message_hash,
                tte.account_workchain_id,
                tte.account_hex,
                tte.owner_message_hash,
                tte.value,
                tte.root_address,
                tte.transaction_direction as "transaction_direction: _",
                tte.transaction_status as "transaction_status: _",
                tte.event_status as "event_status: _",
                tte.created_at,
                tte.updated_at,
                tt.transaction_hash as token_transaction_hash
                FROM token_transaction_events tte
                    LEFT JOIN token_transactions tt on tt.id = tte.token_transaction_id
                WHERE tte.service_id = $1 {} ORDER BY tte.created_at DESC OFFSET ${} LIMIT ${}"#,
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
                owner_message_hash: x.get(6),
                value: x.get(7),
                root_address: x.get(8),
                transaction_direction: x.get(9),
                transaction_status: x.get(10),
                event_status: x.get(11),
                created_at: x.get(12),
                updated_at: x.get(13),
                token_transaction_hash: x.get(14),
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
        owner_message_hash,
        transaction_direction,
        transaction_status,
        event_status,
        ..
    } = input.clone();
    let mut updates = Vec::new();

    if let Some(token_transaction_id) = token_transaction_id {
        updates.push(format!(
            " AND tte.token_transaction_id = ${} ",
            *args_len + 1,
        ));
        *args_len += 1;
        args.add(token_transaction_id)
    }

    if let Some(root_address) = root_address {
        updates.push(format!(" AND tte.root_address = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(root_address)
    }

    if let Some(message_hash) = message_hash {
        updates.push(format!(" AND tte.message_hash = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(message_hash)
    }

    if let Some(account_workchain_id) = account_workchain_id {
        updates.push(format!(
            " AND tte.account_workchain_id = ${} ",
            *args_len + 1,
        ));
        *args_len += 1;
        args.add(account_workchain_id)
    }

    if let Some(account_hex) = account_hex {
        updates.push(format!(" AND tte.account_hex = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(account_hex)
    }

    if let Some(owner_message_hash) = owner_message_hash {
        updates.push(format!(" AND tte.owner_message_hash = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(owner_message_hash)
    }

    if let Some(transaction_direction) = transaction_direction {
        updates.push(format!(
            " AND tte.transaction_direction = ${} ",
            *args_len + 1,
        ));
        *args_len += 1;
        args.add(transaction_direction)
    }

    if let Some(transaction_status) = transaction_status {
        updates.push(format!(" AND tte.transaction_status = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(transaction_status)
    }

    if let Some(event_status) = event_status {
        updates.push(format!(" AND tte.event_status = ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(event_status)
    }

    if let Some(created_at_ge) = created_at_ge {
        updates.push(format!(" AND tte.created_at >= ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(created_at_ge)
    }

    if let Some(created_at_le) = created_at_le {
        updates.push(format!(" AND tte.created_at <= ${} ", *args_len + 1,));
        *args_len += 1;
        args.add(created_at_le)
    }

    updates
}

#[cfg(test)]
mod test {}
