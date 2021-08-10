use anyhow::Context;
use anyhow::Result;
use indexer_lib::TransactionExt;
use nekoton::utils::NoFailure;
use ton_block::{Deserializable, Serializable};

use crate::models::raw_transaction::RawTransactionFromDb;
use crate::prelude::stream::StreamExt;
use crate::sqlx_client::SqlxClient;
use nekoton::utils::TrustMe;
use std::convert::TryInto;
impl SqlxClient {
    pub async fn count_raw_transactions(&self) -> Result<i64> {
        sqlx::query!(r#"SELECT count(*) FROM raw_transactions"#)
            .fetch_one(&self.pool)
            .await
            .map(|x| x.count.unwrap_or_default())
            .map_err(anyhow::Error::new)
    }

    pub async fn insert_raw_transaction(
        &self,
        transaction: &ton_block::Transaction,
        block_hash: &[u8],
    ) -> Result<(), anyhow::Error> {
        let hash = transaction.tx_hash()?.as_slice().to_vec();
        let bytes = transaction
            .write_to_bytes()
            .convert()
            .context("Failed serializing tx to bytes")?;
        sqlx::query!(
            r#"INSERT INTO raw_transactions (hash, transaction, block_time, block_hash) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING"#,
            hash,
            bytes,
            transaction.now as i32,
            block_hash
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_raw_transaction(&self, hash: &[u8]) -> Result<ton_block::Transaction> {
        let res = sqlx::query!(
            r#"SELECT transaction 
            FROM raw_transactions 
            WHERE hash = $1"#,
            hash
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(
            ton_block::Transaction::construct_from_bytes(&res.transaction)
                .convert()
                .context("Failed constructing tx from bytes")?,
        )
    }

    pub async fn stream_raw_transactions(&self) -> Vec<RawTransactionFromDb> {
        sqlx::query!(
            r#"SELECT  transaction, block_hash,block_time
            FROM raw_transactions"#
        )
        .fetch(&self.pool)
        .filter_map(|x| async {
            match x {
                Ok(record) => {
                    let tx = match ton_block::Transaction::construct_from_bytes(
                        record.transaction.as_slice(),
                    ) {
                        Ok(a) => a,
                        Err(e) => {
                            log::error!("Failed constructing tx from db: {}", e);
                            return None;
                        }
                    };
                    Some(RawTransactionFromDb {
                        transaction: tx,
                        block_hash: record.block_hash.try_into().trust_me(),
                        block_time: record.block_time as u32,
                    })
                }
                Err(e) => {
                    log::error!("Db error {}", e);
                    None
                }
            }
        })
        .collect()
        .await
    }
}
