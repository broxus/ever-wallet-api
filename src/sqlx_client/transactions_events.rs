use std::convert::TryInto;

use anyhow::Result;
use indexer_lib::TransactionExt;
use nekoton::utils::{NoFailure, TrustMe};
use ton_block::{Deserializable, Serializable, Transaction};

use crate::prelude::stream::StreamExt;
use crate::sqlx_client::SqlxClient;

impl SqlxClient {
    // pub async fn insert_failed_tx(
    //     &self,
    //     transaction: Transaction,
    //     reason: String,
    //     block_hash: &[u8],
    // ) -> Result<()> {
    //     let block_time = transaction.now;
    //     let hash = transaction.tx_hash()?.as_slice().to_vec();
    //     let transaction = transaction.write_to_bytes().convert()?;
    //     sqlx::query!(
    //         "INSERT INTO bad_transactions
    //         (hash, transaction, reason, block_hash,block_time )
    //         VALUES ($1, $2, $3, $4, $5)
    //         ON CONFLICT DO NOTHING",
    //         hash,
    //         transaction,
    //         reason,
    //         block_hash,
    //         block_time as i32
    //     )
    //     .execute(&self.pool)
    //     .await?;
    //     Ok(())
    // }
    //
    // pub async fn get_bad_transactions(&self) -> Vec<RawTransactionFromDb> {
    //     sqlx::query!(
    //         "SELECT transaction, block_hash,block_time
    //         FROM bad_transactions"
    //     )
    //     .fetch(&self.pool)
    //     .filter_map(|x| async {
    //         match x {
    //             Ok(record) => {
    //                 let tx = match ton_block::Transaction::construct_from_bytes(
    //                     record.transaction.as_slice(),
    //                 ) {
    //                     Ok(a) => a,
    //                     Err(e) => {
    //                         log::error!("Failed constructing tx from db: {}", e);
    //                         return None;
    //                     }
    //                 };
    //                 Some(RawTransactionFromDb {
    //                     transaction: tx,
    //                     block_hash: record.block_hash.try_into().trust_me(),
    //                     block_time: record.block_time as u32,
    //                 })
    //             }
    //             Err(e) => {
    //                 log::error!("Db error {}", e);
    //                 None
    //             }
    //         }
    //     })
    //     .collect()
    //     .await
    // }
}
