use std::sync::Arc;

use anyhow::Result;
use futures::channel::mpsc::Receiver;
use futures::{SinkExt, StreamExt};
use indexer_lib::{
    BounceHandler, ExtractInput, FunctionOpts, ParsedFunctionWithBounce, ParsedOutput,
    TransactionExt,
};
use nekoton::utils::NoFailure;
use node_indexer::NodeClient;
use ton_abi::{Token, TokenValue, Uint};
use ton_block::{Block, GetRepresentationHash};

pub use abi::*;
// pub const START_BLOCK: i32 = 9307323;
pub use functions_history::{ROOT_CONTRACT_HASH, TOKEN_WALLET_CODE_HASH};

use crate::models::owners_cache::OwnersCache;
use crate::models::raw_transaction::RawTransactionFromDb;
use crate::models::root_contracts_cache::RootContractsCache;
use crate::prelude::RedisPool;
use crate::redis::{BlocksRepo, BlocksRepoImpl, RedisExecutorImpl};
use crate::sqlx_client::SqlxClient;
use crate::ws_indexer::abi::{ROOT_TOKEN_CONTRACT, TON_TOKEN_WALLET};
use crate::ws_indexer::functions_history::parse_transactions_functions;
use ton_types::SliceData;

pub const START_BLOCK: i32 = 8602188;

mod abi;
mod functions_history;

pub async fn ton_indexer_stream(
    sqlx_client: SqlxClient,
    owners_hash: OwnersCache,
    contracts_hash: RootContractsCache,
    redis_pool: RedisPool,
) {
    let redis_executor = RedisExecutorImpl::new(redis_pool);
    let sqlx_clone = sqlx_client.clone();
    let owners_hash_clone = owners_hash.clone();
    let contracts_hash_clone = contracts_hash.clone();
    let pool_size = 300;
    let config = node_indexer::Config {
        pool_size,
        ..Default::default()
    };
    let node = Arc::new(NodeClient::new(config).await.unwrap());
    let node_clone = node.clone();
    parse_blocks(
        redis_executor.clone(),
        sqlx_clone,
        owners_hash_clone,
        contracts_hash_clone,
        node_clone,
        pool_size as usize,
    )
    .await;
}

pub type BlockHash = [u8; 32];
fn parsed_blocks_producer(
    mut rx: Receiver<Block>,
) -> futures::channel::mpsc::Receiver<(ParsedOutput<ParsedFunctionWithBounce>, [u8; 32])> {
    let (mut tx, rx_out) = futures::channel::mpsc::channel(256);
    let root_functions = prep_functions();
    tokio::spawn(async move {
        while let Some(a) = rx.next().await {
            let hash: BlockHash = match a.hash() {
                Ok(a) => *a.as_slice(),
                Err(e) => {
                    log::error!("Failed reading block info. Skipping: {:?}", e);
                    continue;
                }
            };

            match indexer_lib::extract_from_block(&a, &root_functions) {
                Ok(functions) => {
                    if !functions.is_empty() {
                        for fun in functions {
                            tx.send((fun, hash)).await.expect("Channel is broken");
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed extracting functions from block: {:?}", e);
                }
            };
        }
    });
    rx_out
}

async fn parse_blocks(
    redis_executor: RedisExecutorImpl,
    sqlx_client: SqlxClient,
    owners_cash: OwnersCache,
    root_contracts_cache: RootContractsCache,
    node: Arc<NodeClient>,
    pool_size: usize,
) {
    let (tx, rx) = futures::channel::mpsc::channel(pool_size * 16);
    let (tx_block_id, mut rx_block_id) =
        futures::channel::mpsc::channel::<ton_api::ton::ton_node::blockid::BlockId>(10);
    let mut block_id = {
        if let Ok(mut redis_conn) = redis_executor.get_connection() {
            let mut repo = BlocksRepoImpl::new(&mut redis_conn);
            repo.get().ok().unwrap_or_default()
        } else {
            None
        }
    }
    .map(|mut x| {
        x.seqno -= (pool_size * 16 * 4) as i32;
        x
    });

    tokio::spawn(async move {
        let mut timer = std::time::Instant::now();
        while let Some(block_id) = rx_block_id.next().await {
            let now = std::time::Instant::now();
            if (now - timer).as_secs() > 60 {
                log::info!("Current block: {}", &block_id.seqno);
                timer = now;
            }
            if let Ok(mut redis_conn) = redis_executor.get_connection() {
                let mut repo = BlocksRepoImpl::new(&mut redis_conn);
                if let Err(e) = repo.set(block_id) {
                    log::error!("Failed writing to redis {:?}", e);
                }
            }
        }
    });
    // restoring failing transactions
    // eg node haven't give us state for account
    let bad = sqlx_client.get_bad_transactions().await;
    log::info!("Restoring {} bad transactions", bad.len());
    restore_transactions(
        &sqlx_client,
        &node,
        &owners_cash,
        &root_contracts_cache,
        bad,
    )
    .await;
    log::info!("Finished restoring bad transactions");
    if sqlx_client.count_all_transactions().await.unwrap() < 10
        && !restore(&sqlx_client, &node, &owners_cash, &root_contracts_cache)
            .await
            .unwrap()
    {
        block_id = Some(ton_api::ton::ton_node::blockid::BlockId {
            workchain: -1,
            shard: u64::from_str_radix("8000000000000000", 16).unwrap() as i64,
            seqno: START_BLOCK,
        });
    }

    log::info!("Starting indexer from {:?}", block_id);
    node.spawn_indexer(block_id, tx, tx_block_id).await.unwrap();
    let mut rx = parsed_blocks_producer(rx);
    while let Some((parsed, hash)) = rx.next().await {
        let tx_hash = hex::encode(parsed.hash.as_slice());
        let tx = parsed.transaction.clone();
        if let Err(e) = parse_transactions_functions(
            parsed,
            &node,
            &sqlx_client,
            &owners_cash,
            &root_contracts_cache,
            hash,
        )
        .await
        {
            log::error!("Failed parsing: {:?}. Tx hash: {}", e, tx_hash);
            let error = e.to_string();
            sqlx_client
                .insert_failed_tx(tx, error, &hash)
                .await
                .unwrap();
        }
    }
    panic!("Indexer stream is finished");
}

/// returns true, if there some raw transactions and we don't to fully rescan
async fn restore(
    sqlx_client: &SqlxClient,
    node: &NodeClient,
    owners_cache: &OwnersCache,
    contracts_cache: &RootContractsCache,
) -> Result<bool> {
    let res = if sqlx_client.count_raw_transactions().await? != 0 {
        log::warn!("Starting restore");
        let transactions = sqlx_client.stream_raw_transactions().await;
        log::info!("Restoring from {} raw transactions", transactions.len());
        restore_transactions(
            sqlx_client,
            node,
            owners_cache,
            contracts_cache,
            transactions,
        )
        .await;
        true
    } else {
        false
    };
    log::info!("Finished restoring");
    Ok(res)
}

async fn restore_transactions(
    sqlx_client: &SqlxClient,
    node: &NodeClient,
    owners_cash: &OwnersCache,
    contracts_cash: &RootContractsCache,
    transactions: Vec<RawTransactionFromDb>,
) {
    let funs = prep_functions();

    for raw_tx_from_db in transactions {
        let tx = raw_tx_from_db.transaction;
        let tx_hash = match tx.tx_hash() {
            Ok(a) => a,
            Err(e) => {
                log::error!("Failed calculating tx hash: {}", e);
                continue;
            }
        };
        let res = ExtractInput {
            transaction: &tx,
            hash: tx_hash,
            what_to_extract: &funs,
        }
        .process();
        let output = match res {
            Ok(Some(a)) => a,
            Err(e) => {
                log::error!("Failed parsing tx: {}", e);
                if let Err(e) = sqlx_client
                    .insert_failed_tx(tx, e.to_string(), &raw_tx_from_db.block_hash)
                    .await
                {
                    log::error!("Failed writing failed tx in db: {}", e);
                }
                continue;
            }
            _ => continue,
        };
        if let Err(e) = parse_transactions_functions(
            output,
            node,
            sqlx_client,
            owners_cash,
            contracts_cash,
            raw_tx_from_db.block_hash,
        )
        .await
        {
            log::error!("Failed enriching parsed data: {:?}", e);
            if let Err(e) = sqlx_client
                .insert_failed_tx(tx, e.to_string(), &raw_tx_from_db.block_hash)
                .await
            {
                log::error!("Failed writing failed tx in db: {}", e);
            }
        };
    }
}

fn bounce_handler(mut data: SliceData) -> Result<Vec<Token>> {
    let _id = data.get_next_u32().convert()?;
    let token = data.get_next_u128().convert()?;
    Ok(vec![Token::new(
        "tokens",
        TokenValue::Uint(Uint::new(token, 128)),
    )])
}

fn prep_functions() -> [FunctionOpts<BounceHandler>; 3] {
    let wallet_contract = ton_abi::Contract::load(std::io::Cursor::new(TON_TOKEN_WALLET)).unwrap();
    let root_contract = ton_abi::Contract::load(std::io::Cursor::new(ROOT_TOKEN_CONTRACT)).unwrap();
    let wallet_fns = wallet_contract.functions();
    let internal_transfer = wallet_fns.get("internalTransfer").unwrap();
    let parse_function2 = wallet_fns.get("accept").unwrap();
    let tokens_burned = root_contract.functions().get("tokensBurned").unwrap();
    let handler = bounce_handler as BounceHandler;
    let transfer = FunctionOpts {
        function: internal_transfer.clone(),
        handler: Some(handler),
        match_outgoing: true,
    };
    let burn = FunctionOpts {
        function: tokens_burned.clone(),
        handler: Some(handler),
        match_outgoing: false,
    };

    [transfer, parse_function2.clone().into(), burn]
}

#[cfg(test)]
mod test {
    use indexer_lib::{ExtractInput, TransactionExt};
    use node_indexer::{Config, NodeClient};
    use sqlx::PgPool;

    use crate::models::owners_cache::OwnersCache;
    use crate::models::root_contracts_cache::RootContractsCache;
    use crate::sqlx_client::SqlxClient;
    use crate::ws_indexer::prep_functions;

    async fn init() -> (NodeClient, SqlxClient, OwnersCache, RootContractsCache) {
        let node = NodeClient::new(Config::default()).await.unwrap();
        let db = SqlxClient::new(
            PgPool::connect(
                "postgresql://postgres:postgres@localhost:5432/trading_ton_wallet_api_rs",
            )
            .await
            .unwrap(),
        );
        let owners = OwnersCache::new(db.clone()).await.unwrap();
        let contracts = RootContractsCache::new(db.clone()).await.unwrap();
        (node, db, owners, contracts)
    }

    #[tokio::test]
    async fn find_burn() {
        let (_node, db, _owners, _contracts) = init().await;
        let txs = db.stream_raw_transactions().await;
        let funs = prep_functions();
        for tx in txs {
            let tx = tx.transaction;
            let tx_hash = match tx.tx_hash() {
                Ok(a) => a,
                Err(e) => {
                    log::error!("Failed calculating tx hash: {}", e);
                    continue;
                }
            };
            let res = ExtractInput {
                transaction: &tx,
                hash: tx_hash,
                what_to_extract: &funs,
            }
            .process();
            let output = match res {
                Ok(Some(a)) => a,
                _ => continue,
            };
            if output.output[0].function_name.to_lowercase() == "TokensBurned".to_lowercase() {
                println!("{}", hex::encode(tx_hash.as_slice()));
                break;
            }
        }
    }

    #[tokio::test]
    async fn test_restore() {
        let (node, db, owners, contracts) = init().await;

        super::restore(&db, &node, &owners, &contracts).await;
    }
}
