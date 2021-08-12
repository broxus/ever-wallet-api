use std::str::FromStr;

use anyhow::Result;
use itertools::Itertools;
use sqlx::postgres::PgArguments;
use sqlx::Arguments;
use sqlx::Row;

use crate::models::sqlx::{TransactionFromDb, TransactionToDb};
use crate::sqlx_client::SqlxClient;

impl SqlxClient {
    // pub async fn count_all_transactions(&self) -> Result<i64> {
    //     sqlx::query!(r#"SELECT count(*) FROM transactions"#)
    //         .fetch_one(&self.pool)
    //         .await
    //         .map(|x| x.count.unwrap_or_default())
    //         .map_err(anyhow::Error::new)
    // }
    //
    // pub async fn find_transactions(
    //     &self,
    //     input: &TransactionsSearch,
    // ) -> Result<Vec<TransactionFromDb>, anyhow::Error> {
    //     let TransactionsSearch {
    //         ordering,
    //         limit,
    //         offset,
    //         ..
    //     } = input.clone();
    //
    //     let limit = if limit > 5000 { 5000 } else { limit };
    //
    //     let (params, mut args, args_len) = query_filter_builder(input);
    //
    //     let mut query = format!(
    //         "SELECT transaction_hash, message_hash, owner_address, token_wallet_address, public_key, amount, root_address, token, kind, meta, payload, callback_address, failed_reason, block_hash, block_time, created_at
    //          FROM transactions {}", params
    //     );
    //
    //     if let Some(ordering) = ordering {
    //         let ordering = match ordering {
    //             TransactionsOrdering::BlockTimeAtAscending => "ORDER BY block_time",
    //             TransactionsOrdering::BlockTimeAtDescending => "ORDER BY block_time DESC",
    //         };
    //         query = format!(
    //             "{} {} OFFSET ${} LIMIT ${}",
    //             query,
    //             ordering,
    //             args_len + 1,
    //             args_len + 2
    //         );
    //     } else {
    //         query = format!(
    //             "{} ORDER BY block_time DESC OFFSET ${} LIMIT ${}",
    //             query,
    //             args_len + 1,
    //             args_len + 2
    //         );
    //     }
    //     args.add(offset);
    //     args.add(limit);
    //
    //     log::debug!("query - {}", query);
    //
    //     let transactions = sqlx::query_with(&query, args).fetch_all(&self.pool).await?;
    //
    //     let res = transactions
    //         .iter()
    //         .map(|x| TransactionFromDb {
    //             transaction_hash: x.get(0),
    //             message_hash: x.get(1),
    //             owner_address: x.get(2),
    //             token_wallet_address: x.get(3),
    //             public_key: x.get(4),
    //             amount: x.get(5),
    //             root_address: x.get(6),
    //             token: x.get(7),
    //             kind: (TransactionKind::from_str(&x.get::<String, _>(8)).unwrap()),
    //             meta: x.get(9),
    //             payload: x.get(10),
    //             callback_address: x.get(11),
    //             failed_reason: x.get(12),
    //             block_hash: x.get(13),
    //             block_time: (x.get::<i32, _>(14) as u32),
    //             created_at: x.get(15),
    //         })
    //         .collect::<Vec<_>>();
    //
    //     Ok(res)
    // }
    //
    // pub async fn count_transactions(
    //     &self,
    //     input: &TransactionsSearch,
    // ) -> Result<i64, anyhow::Error> {
    //     let (params, args, _args_len) = query_filter_builder(input);
    //
    //     let query = format!("SELECT COUNT(*) FROM transactions {}", params);
    //
    //     log::debug!("query - {}", query);
    //
    //     sqlx::query_with(&query, args)
    //         .fetch_one(&self.pool)
    //         .await
    //         .map(|x| x.get::<i64, usize>(0))
    //         .map_err(anyhow::Error::new)
    // }
    //
    // pub async fn new_transaction(&self, transaction: TransactionToDb) -> Result<()> {
    //     let failed_reason = transaction
    //         .failed_reason
    //         .as_ref()
    //         .map(|a| a.iter().map(|x| x.to_string()).join(","));
    //     let meta = serde_json::to_value(&transaction.meta)?;
    //     let res =
    //             sqlx::query!(get_address_balance
    //             r#"INSERT INTO trading_ton_wallet_api_rs.public.transactions
    //             (transaction_hash, message_hash, owner_address, token_wallet_address, public_key, amount, root_address, token, kind, meta, payload, callback_address, failed_reason,  block_hash, block_time)
    //             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)"#,
    //             transaction.transaction_hash,
    //             transaction.message_hash,
    //             transaction.owner_address,
    //             transaction.token_wallet_address,
    //             transaction.public_key.clone().unwrap_or_else(||vec![0;32]),
    //             transaction.amount,
    //             transaction.root_address,
    //             transaction.token,
    //             transaction.kind.to_string(),
    //             meta,
    //             transaction.payload,
    //             transaction.callback_address,
    //             failed_reason,
    //             transaction.block_hash,
    //             transaction.block_time as i32,
    //         )
    //                 .execute(&self.pool)
    //                 .await;
    //
    //     if let Err(e) = res {
    //         log::error!("Failed inserting transaction: {}", e)
    //     };
    //
    //     Ok(())
    // }
}

// fn query_filter_builder(input: &TransactionsSearch) -> (String, PgArguments, i32) {
//     let TransactionsSearch {
//         token,
//         root_address,
//         kind,
//         owner_address,
//         public_key,
//         block_time_ge,
//         block_time_le,
//         transaction_hash,
//         message_hash,
//         ..
//     } = input.clone();
//     let mut args = PgArguments::default();
//     let mut params = Vec::new();
//     let mut args_len = 0;
//
//     if let Some(kind) = kind {
//         params.push(format!(" kind = ANY(${}) ", args_len + 1,));
//         args_len += 1;
//         args.add(kind.into_iter().map(|x| x.to_string()).collect::<Vec<_>>())
//     };
//
//     if let Some(token) = token {
//         params.push(format!("token = ${} ", args_len + 1,));
//         args_len += 1;
//         args.add(token);
//     }
//
//     if let Some(transaction_hash) = transaction_hash {
//         params.push(format!("transaction_hash = ${} ", args_len + 1,));
//         args_len += 1;
//         args.add(transaction_hash);
//     }
//
//     if let Some(message_hash) = message_hash {
//         params.push(format!("message_hash = ${} ", args_len + 1,));
//         args_len += 1;
//         args.add(message_hash);
//     }
//
//     if let Some(root_address) = root_address {
//         params.push(format!(" root_address = ANY(${}) ", args_len + 1,));
//         args_len += 1;
//         args.add(root_address)
//     }
//
//     if let Some(owner_address) = owner_address {
//         params.push(format!(" owner_address = ${} ", args_len + 1,));
//         args_len += 1;
//         args.add(owner_address)
//     }
//
//     if let Some(block_time_ge) = block_time_ge {
//         params.push(format!(" block_time >= ${} ", args_len + 1,));
//         args_len += 1;
//         args.add(block_time_ge)
//     }
//
//     if let Some(block_time_le) = block_time_le {
//         params.push(format!("  block_time <= ${} ", args_len + 1,));
//         args_len += 1;
//         args.add(block_time_le)
//     }
//
//     if let Some(public_key) = public_key {
//         params.push(format!(" public_key = ${} ", args_len + 1,));
//         args_len += 1;
//         args.add(public_key)
//     }
//
//     let params = if let Some((first, elements)) = params.split_first() {
//         elements.iter().fold(format!("WHERE {}", first), |acc, x| {
//             format!("{} AND {}", acc, x)
//         })
//     } else {
//         "".to_string()
//     };
//
//     (params, args, args_len)
// }

#[cfg(test)]
mod test {
    // use crate::models::transaction_kind::TransactionKind;
    // use crate::models::transactions::TransactionsSearch;
    // use crate::models::transactions_ordering::TransactionsOrdering;
    // use crate::sqlx_client::SqlxClient;
    // use sqlx::PgPool;
    //
    // #[tokio::test]
    // async fn test() {
    //     let pg_pool = PgPool::connect(
    //         "postgresql://postgres:postgres@localhost:5432/trading_ton_wallet_api_rs",
    //     )
    //     .await
    //     .unwrap();
    //     let sqlx_client = SqlxClient::new(pg_pool);
    //     sqlx_client
    //         .find_transactions(&TransactionsSearch {
    //             ordering: Some(TransactionsOrdering::BlockTimeAtAscending),
    //             limit: 0,
    //             offset: 10,
    //             token: Some("USDT".to_string()),
    //             root_address: Some(vec!["test".to_string(), "foo".to_string()]),
    //             kind: None,
    //             owner_address: Some("test_owner".to_string()),
    //             public_key: Some(vec![]),
    //             transaction_hash: Some(vec![]),
    //             message_hash: Some(vec![]),
    //             block_time_ge: Some(0),
    //             block_time_le: Some(1000),
    //         })
    //         .await
    //         .unwrap();
    // }
}
