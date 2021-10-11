use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::mpsc;
use ton_types::UInt256;

use crate::models::*;
use crate::settings::*;
use crate::sqlx_client::*;
use crate::ton_core::*;

pub async fn add_root_token(
    config: AppConfig,
    global_config: ton_indexer::GlobalConfig,
    token_name: String,
    token_address: String,
) -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(config.db_pool_size)
        .connect(&config.database_url)
        .await
        .expect("fail pg pool");

    let sqlx_client = SqlxClient::new(pool);
    let owners_cache = OwnersCache::new(sqlx_client.clone()).await?;

    let (caught_ton_transaction_tx, _) = mpsc::unbounded_channel();
    let (caught_token_transaction_tx, _) = mpsc::unbounded_channel();

    let ton_core = TonCore::new(
        config.ton_core,
        global_config,
        sqlx_client.clone(),
        owners_cache.clone(),
        caught_ton_transaction_tx,
        caught_token_transaction_tx,
    )
    .await?;
    ton_core.start().await?;

    let address = nekoton_utils::repack_address(&token_address)?;
    let account = UInt256::from_be_bytes(&address.address().get_bytestring(0));

    let contract = loop {
        match ton_core.get_contract_state(&account) {
            Ok(contract) => {
                break contract;
            }
            Err(_) => {
                const TIME_TO_SLEEP: u64 = 1; // sec
                tokio::time::sleep(std::time::Duration::from_secs(TIME_TO_SLEEP)).await;
                continue;
            }
        };
    };

    let contract = serde_json::to_value(contract)?;

    sqlx_client
        .create_root_token(TokenWhitelistFromDb {
            name: token_name,
            address: token_address,
            contract,
        })
        .await?;

    log::info!("Root token has been added!");

    Ok(())
}
