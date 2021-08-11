use std::time::Duration;

use anyhow::{Context, Result};
use bigdecimal::BigDecimal;
use indexer_lib::{ParsedFunctionWithBounce, ParsedOutput, TransactionExt};
use nekoton::abi::UnpackToken;
use nekoton::core::models::{RootTokenContractDetails, TokenWalletDetails};
use nekoton::core::token_wallet::RootTokenContractState;
use nekoton::transport::models::RawContractState;
use nekoton::utils::NoFailure;
use node_indexer::NodeClient;
use ton_block::{AccountStuff, GetRepresentationHash, MsgAddressInt, Serializable};

use crate::models::owners_cache::{OwnerInfo, OwnersCache};
use crate::models::root_contracts_cache::{RootContractInfo, RootContractsCache};
use crate::models::sqlx::{FailedReason, TransactionMetaDb, TransactionToDb};
use crate::models::transaction_kind::TransactionKind;
use crate::sqlx_client::SqlxClient;
use crate::ws_indexer::functions_history::models::{
    Accept, InternalTransfer, InternalTransferBounced, TokensBurned, TokensBurnedBounced,
};

mod models;

pub const TOKEN_WALLET_CODE_HASH: [u8; 32] = [
    44, 127, 188, 81, 97, 200, 223, 145, 75, 25, 193, 126, 27, 104, 81, 113, 32, 159, 175, 201, 32,
    0, 153, 178, 193, 252, 136, 125, 89, 93, 42, 227,
];

pub const ROOT_CONTRACT_HASH: [u8; 32] = [
    59, 174, 74, 40, 163, 73, 26, 163, 72, 173, 155, 79, 33, 202, 100, 40, 40, 252, 236, 176, 164,
    148, 82, 70, 197, 186, 122, 212, 247, 248, 125, 4,
];

fn enrich_with_error(mut transaction: TransactionToDb, error: FailedReason) -> TransactionToDb {
    let reason = match transaction.failed_reason {
        None => {
            vec![error]
        }
        Some(mut a) => {
            a.push(error);
            a
        }
    };
    transaction.failed_reason = Some(reason);
    transaction
}

pub async fn parse_transactions_functions(
    value: ParsedOutput<ParsedFunctionWithBounce>,
    node: &NodeClient,
    sqlx_client: &SqlxClient,
    contracts_cache: &RootContractsCache,
    block_hash: [u8; 32],
) -> Result<()> {
    let now = chrono::Utc::now();
    anyhow::ensure!(!value.output.is_empty(), "Empty parsing output");
    let transaction = value.transaction.clone();
    let parse_context = ParseContext {
        node,
        owners_cache,
        root_contracts_cache: contracts_cache,
    };
    let transaction_to_db = match value.output[0].function_name.as_str() {
        "internalTransfer" => map_internal_transfer(value, block_hash, parse_context).await,
        "accept" => map_mint(value, block_hash, parse_context).await.map(Some),
        "tokensBurned" => map_burn(value, block_hash, parse_context).await.map(Some),
        _ => anyhow::bail!("invalid function name"),
    }?;
    let transaction_to_db = match transaction_to_db {
        None => return Ok(()),
        Some(a) => a,
    };
    let bad_hash = match transaction_to_db.failed_reason.as_ref() {
        None => false,
        Some(a) => a.iter().any(|x| x == &FailedReason::BadHash),
    };

    if !bad_hash {
        sqlx_client
            .new_transaction(transaction_to_db)
            .await
            .context("Failed writing parsed transaction to db")?;

        if let Err(e) = sqlx_client
            .insert_raw_transaction(&transaction, &block_hash)
            .await
        {
            log::error!("failed inserting raw transaction: {}", e)
        };
        let took = chrono::Utc::now() - now;
        log::info!(
            "Parsing took {}. Processed hash: {}",
            took,
            hex::encode(&transaction.tx_hash()?.as_slice())
        );
    }
    Ok(())
}

struct ParseContext<'a> {
    node: &'a NodeClient,
    owners_cache: &'a OwnersCache,
    root_contracts_cache: &'a RootContractsCache,
}

async fn internal_transfer_bounced(
    mut value: ParsedOutput<ParsedFunctionWithBounce>,
    block_hash: [u8; 32],
    ctx: ParseContext<'_>,
) -> Result<TransactionToDb> {
    let function = value.output.remove(0);
    let token_wallet = value.transaction.contract_address()?;
    let receiver_info = get_receiver_info(
        token_wallet.clone(),
        ctx.node,
        ctx.owners_cache,
        ctx.root_contracts_cache,
    )
    .await
    .context("Failed getting info about token wallet in bounce")?;
    let input = function.input.context("No function input")?;

    let parsed: InternalTransferBounced = input
        .tokens
        .unpack()
        .context("Failed parsing output to internal transfer")?;
    let amount = BigDecimal::new(parsed.tokens.into(), receiver_info.scale as i64);
    let (transaction_hash, block_time, message_hash) = (
        value.transaction.tx_hash()?.as_slice().to_vec(),
        value.transaction.now,
        input.hash.to_vec(),
    );
    let cancellation = TransactionToDb {
        kind: TransactionKind::SendCancellation,
        transaction_hash,
        message_hash,
        owner_address: receiver_info.owner_address.to_string(),
        token_wallet_address: token_wallet.to_string(),
        public_key: receiver_info.owner_public_key,
        amount,
        root_address: receiver_info.root_address.to_string(),
        token: receiver_info.token,
        meta: None,
        payload: None,
        callback_address: None,
        failed_reason: None,
        block_hash: block_hash.to_vec(),
        block_time,
    };
    Ok(
        if TOKEN_WALLET_CODE_HASH.as_ref() != receiver_info.code_hash {
            enrich_with_error(cancellation, FailedReason::BadHash)
        } else {
            cancellation
        },
    )
}

async fn map_internal_transfer_send(
    value: ParsedOutput<ParsedFunctionWithBounce>,
    block_hash: [u8; 32],
    ctx: ParseContext<'_>,
) -> Result<TransactionToDb> {
    let function = &value.output[0];
    let sender_token_wallet_address = value.transaction.contract_address()?;
    let sender_info = get_receiver_info(
        sender_token_wallet_address.clone(),
        ctx.node,
        ctx.owners_cache,
        ctx.root_contracts_cache,
    )
    .await
    .context("Failed getting info about sender")?;
    let output = function.output.clone().context("No function output")?;

    let parsed: InternalTransfer = output
        .tokens
        .unpack()
        .context("Failed parsing output to internal transfer")?;
    let amount = BigDecimal::new(parsed.tokens.into(), sender_info.scale as i64);
    let payload = parsed
        .payload
        .write_to_bytes()
        .convert()
        .context("Failed serializing payload")?;
    let root_address = sender_info.root_address.to_string();
    let token = sender_info.token;
    let block_hash = block_hash.to_vec();
    let message_hash = output.hash;
    let receiver_token_wallet_address = value
        .transaction
        .messages()?
        .out_messages
        .iter()
        .find(|x| {
            x.msg
                .hash()
                .expect("If transaction parsed, than message is ok")
                .as_slice()
                == &message_hash
        })
        .expect("We matched this tx. So message exists")
        .msg
        .dst()
        .expect("Internal transfer always has dst");

    let (receiver_owner_address, receiver_pubkey) = get_receiver_info(
        receiver_token_wallet_address.clone(),
        ctx.node,
        ctx.owners_cache,
        ctx.root_contracts_cache,
    )
    .await
    .map(|x| (x.owner_address, x.owner_public_key))
    .unwrap_or((receiver_token_wallet_address, None));

    let (transaction_hash, block_time) = (
        value.transaction.tx_hash()?.as_slice().to_vec(),
        value.transaction.now,
    );
    let send = TransactionToDb {
        kind: TransactionKind::Send,
        transaction_hash,
        message_hash: message_hash.to_vec(),
        owner_address: sender_info.owner_address.to_string(),
        token_wallet_address: sender_token_wallet_address.to_string(),
        public_key: sender_info.owner_public_key,
        amount: -amount,
        root_address,
        token,
        meta: Some(TransactionMetaDb::Receiver {
            receiver_address: receiver_owner_address.to_string(),
            receiver_public_key: receiver_pubkey,
        }),
        payload: Some(payload),
        callback_address: None,
        failed_reason: None,
        block_hash,
        block_time,
    };
    Ok(
        if TOKEN_WALLET_CODE_HASH.as_ref() != sender_info.code_hash {
            enrich_with_error(send, FailedReason::BadHash)
        } else {
            send
        },
    )
}

async fn map_internal_transfer_receive(
    value: ParsedOutput<ParsedFunctionWithBounce>,
    block_hash: [u8; 32],
    ctx: ParseContext<'_>,
) -> Result<Option<TransactionToDb>> {
    let function = &value.output[0];
    if !value.transaction.success() {
        return Ok(None);
    }
    let input = function.input.clone().context("No function input")?;
    let parsed: InternalTransfer = input
        .tokens
        .unpack()
        .context("Failed parsing output to internal transfer")?;

    let sender_address = parsed.sender_address.to_string();
    let sender_pubkey = Some(parsed.sender_public_key.as_slice().to_vec());
    let payload = parsed
        .payload
        .write_to_bytes()
        .convert()
        .context("Failed serializing payload")?;

    let receiver_address = value.transaction.contract_address()?;
    let receiver_info = match get_receiver_info(
        receiver_address.clone(),
        ctx.node,
        ctx.owners_cache,
        ctx.root_contracts_cache,
    )
    .await
    {
        Ok(a) => a,
        Err(e) => {
            log::warn!("{} bad receiver info: {}", receiver_address, e);
            return Ok(None);
        }
    };

    let amount = BigDecimal::new(parsed.tokens.into(), receiver_info.scale as i64);

    let root_address = receiver_info.root_address.to_string();
    let token = receiver_info.token;
    let block_hash = block_hash.to_vec();

    let (transaction_hash, block_time) = (
        value.transaction.tx_hash()?.as_slice().to_vec(),
        value.transaction.now,
    );
    let receive = TransactionToDb {
        kind: TransactionKind::Receive,
        transaction_hash,
        message_hash: input.hash.to_vec(),
        owner_address: receiver_info.owner_address.to_string(),
        token_wallet_address: receiver_address.to_string(),
        public_key: receiver_info.owner_public_key,
        amount,
        root_address,
        token,
        meta: Some(TransactionMetaDb::Sender {
            sender_address,
            sender_public_key: sender_pubkey,
        }),
        payload: Some(payload),
        callback_address: None,
        failed_reason: None,
        block_hash,
        block_time,
    };
    Ok(Some(
        if TOKEN_WALLET_CODE_HASH.as_ref() != receiver_info.code_hash {
            enrich_with_error(receive, FailedReason::BadHash)
        } else {
            receive
        },
    ))
}
async fn map_internal_transfer(
    value: ParsedOutput<ParsedFunctionWithBounce>,
    block_hash: [u8; 32],
    ctx: ParseContext<'_>,
) -> Result<Option<TransactionToDb>> {
    //checked by caller
    let function = &value.output[0];
    if function.bounced {
        return internal_transfer_bounced(value, block_hash, ctx)
            .await
            .map(Some);
    }

    return if function.is_outgoing {
        map_internal_transfer_send(value, block_hash, ctx)
            .await
            .map(Some)
    } else {
        map_internal_transfer_receive(value, block_hash, ctx).await
    };
}

async fn map_mint(
    mut value: ParsedOutput<ParsedFunctionWithBounce>,
    block_hash: [u8; 32],
    ctx: ParseContext<'_>,
) -> Result<TransactionToDb> {
    //checked by caller
    let function = value.output.remove(0);

    let token_wallet_address = value.transaction.contract_address()?;
    let receiver_info = get_receiver_info(
        token_wallet_address.clone(),
        ctx.node,
        ctx.owners_cache,
        ctx.root_contracts_cache,
    )
    .await
    .context("Strange receiver info for mint")?;
    let input = function.input.context("No function input")?;
    let parsed: Accept = input
        .tokens
        .unpack()
        .context("Failed parsing output to mint")?;

    let amount = BigDecimal::new(parsed.tokens.into(), receiver_info.scale as i64);

    let root_address = receiver_info.root_address.to_string();
    let token = receiver_info.token;
    let block_hash = block_hash.to_vec();

    let (transaction_hash, block_time) = (
        value.transaction.tx_hash()?.as_slice().to_vec(),
        value.transaction.now,
    );

    let mint = TransactionToDb {
        kind: TransactionKind::Mint,
        transaction_hash,
        message_hash: input.hash.to_vec(),
        owner_address: receiver_info.owner_address.to_string(),
        token_wallet_address: token_wallet_address.to_string(),
        public_key: receiver_info.owner_public_key,
        amount,
        root_address,
        token,
        meta: None,
        payload: None,
        callback_address: None,
        failed_reason: None,
        block_hash,
        block_time,
    };
    let mint = match value.transaction.success() {
        true => mint,
        false => enrich_with_error(mint, FailedReason::FailedTx),
    };

    Ok(
        if TOKEN_WALLET_CODE_HASH.as_ref() != receiver_info.code_hash {
            enrich_with_error(mint, FailedReason::BadHash)
        } else {
            mint
        },
    )
}

async fn burn_bounced(
    value: ParsedOutput<ParsedFunctionWithBounce>,
    block_hash: [u8; 32],
    ctx: ParseContext<'_>,
) -> Result<TransactionToDb> {
    let function = &value.output[0];
    let input = function.input.clone().context("No function input")?;
    let parsed: TokensBurnedBounced = input
        .tokens
        .unpack()
        .context("Failed parsing output to burn")?;
    let token_wallet_address = value.transaction.contract_address()?;
    let owner_details = get_receiver_info(
        token_wallet_address.clone(),
        ctx.node,
        ctx.owners_cache,
        ctx.root_contracts_cache,
    )
    .await
    .context("Failed getting info about token wallet in bounce")?;

    let amount = BigDecimal::new(parsed.tokens.into(), owner_details.scale as i64);

    let block_hash = block_hash.to_vec();

    let (transaction_hash, block_time) = (
        value.transaction.tx_hash()?.as_slice().to_vec(),
        value.transaction.now,
    );

    let burn_cancel = TransactionToDb {
        kind: TransactionKind::BurnCancellation,
        transaction_hash,
        message_hash: input.hash.to_vec(),
        owner_address: owner_details.owner_address.to_string(),
        token_wallet_address: token_wallet_address.to_string(),
        public_key: owner_details.owner_public_key,
        amount,
        root_address: owner_details.root_address.to_string(),
        token: owner_details.token,
        meta: None,
        payload: None,
        callback_address: None,
        failed_reason: None,
        block_hash,
        block_time,
    };
    Ok(burn_cancel)
}

async fn map_burn(
    value: ParsedOutput<ParsedFunctionWithBounce>,
    block_hash: [u8; 32],
    ctx: ParseContext<'_>,
) -> Result<TransactionToDb> {
    //checked by caller
    let function = &value.output[0];
    if function.bounced {
        return burn_bounced(value, block_hash, ctx).await;
    }
    let input = function.input.clone().context("No function input")?;
    let parsed: TokensBurned = input
        .tokens
        .unpack()
        .context("Failed parsing output to burn")?;

    let owner_address = parsed.sender_address;
    let pubkey = Some(parsed.sender_public_key.as_slice().to_vec());
    let root_address = value.transaction.contract_address()?;
    let root_info = match ctx.root_contracts_cache.get(&root_address).await {
        Some(a) => a,
        None => {
            let root_info = get_root_details(ctx.node, root_address.clone()).await?;
            let info = RootContractInfo {
                code_hash: root_info.0,
                state: root_info.1,
                token: root_info.2.symbol,
                scale: root_info.2.decimals as i32,
                name: root_info.2.name,
                owner_address: root_info.2.owner_address,
                root_public_key: root_info.2.root_public_key.as_slice().to_vec(),
            };
            ctx.root_contracts_cache
                .insert(root_address.clone(), info.clone())
                .await;
            info
        }
    };

    let token_wallet_address = root_info
        .calculate_wallet_address(&owner_address, &pubkey)
        .context("Failed deriving token wallet address")?;

    let amount = BigDecimal::new(parsed.tokens.into(), root_info.scale as i64);

    let block_hash = block_hash.to_vec();

    let (transaction_hash, block_time) = (
        value.transaction.tx_hash()?.as_slice().to_vec(),
        value.transaction.now,
    );

    let burn = TransactionToDb {
        kind: TransactionKind::Burn,
        transaction_hash,
        message_hash: input.hash.to_vec(),
        owner_address: owner_address.to_string(),
        token_wallet_address: token_wallet_address.to_string(),
        public_key: pubkey,
        amount: -amount,
        root_address: root_address.to_string(),
        token: root_info.token,
        meta: None,
        payload: None,
        callback_address: None,
        failed_reason: None,
        block_hash,
        block_time,
    };

    let res = match value.transaction.success() {
        true => burn,
        false => enrich_with_error(burn, FailedReason::FailedTx),
    };
    Ok(if ROOT_CONTRACT_HASH.as_ref() != root_info.code_hash {
        enrich_with_error(res, FailedReason::BadHash)
    } else {
        res
    })
}

async fn get_receiver_info(
    contract_address: MsgAddressInt,
    node: &NodeClient,
    owners_cache: &OwnersCache,
    contracts_cache: &RootContractsCache,
) -> Result<OwnerInfo> {
    let res = match owners_cache.get(&contract_address).await {
        None => {
            let (wallet, hash) = get_token_wallet_details(node, contract_address.clone()).await?;
            let root_info = match contracts_cache.get(&wallet.root_address).await {
                None => {
                    let (root_hash, code, root_details) =
                        get_root_details(node, wallet.root_address.clone()).await?;
                    let root_contract_info = RootContractInfo {
                        code_hash: root_hash,
                        token: root_details.symbol,
                        scale: root_details.decimals as i32,
                        name: root_details.name,
                        owner_address: root_details.owner_address,
                        root_public_key: root_details.root_public_key.as_slice().to_vec(),
                        state: code,
                    };
                    contracts_cache
                        .insert(wallet.root_address.clone(), root_contract_info.clone())
                        .await;
                    root_contract_info
                }
                Some(a) => a,
            };
            let info = OwnerInfo {
                owner_address: wallet.owner_address,
                owner_public_key: Some(wallet.wallet_public_key.as_slice().to_vec()),
                root_address: wallet.root_address,
                code_hash: hash.to_vec(),
                token: root_info.token,
                scale: root_info.scale,
            };
            owners_cache.insert(contract_address, info.clone()).await;
            info
        }
        Some(a) => a,
    };
    Ok(res)
}

async fn get_token_wallet_details(
    node: &NodeClient,
    address: MsgAddressInt,
) -> Result<(TokenWalletDetails, [u8; 32])> {
    let state = get_account_state_with_retries(node, &address).await?;
    let state = match state {
        RawContractState::NotExists => {
            anyhow::bail!("Token contract {}  doesn't exist", address.to_string())
        }
        RawContractState::Exists(a) => a,
    };
    let state = nekoton::core::token_wallet::TokenWalletContractState(&state);
    let hash = *state.get_code_hash()?.as_slice();
    let version = state.get_version()?;
    let details = state.get_details(version)?;
    Ok((details, hash))
}

/// Returns hash, state, details
async fn get_root_details(
    node: &NodeClient,
    address: MsgAddressInt,
) -> Result<(Vec<u8>, Vec<u8>, RootTokenContractDetails)> {
    let state = get_account_state_with_retries(node, &address).await?;
    let state = match state {
        RawContractState::NotExists => {
            anyhow::bail!("{} contracts doesn't exist", &address)
        }
        RawContractState::Exists(a) => a,
    };
    let serialized_state = state
        .account
        .write_to_bytes()
        .convert()
        .context("Failed serializing contract")?;
    let hash = calculate_code_hash(&state.account)?.as_slice().to_vec();
    let state = RootTokenContractState(&state);
    Ok((
        hash,
        serialized_state,
        state.guess_details().context("Failed guessing details")?,
    ))
}

async fn get_account_state_with_retries(
    node: &NodeClient,
    address: &MsgAddressInt,
) -> Result<RawContractState> {
    let mut cntr = 0;
    let state = loop {
        cntr += 1;
        let res = node.get_contract_state(address.clone()).await;
        match res {
            Ok(a) => break a,
            Err(e) => {
                if cntr == 100 {
                    return Err(
                        e.context(format!("Failed getting state for {}", address.to_string()))
                    );
                }
                let err = format!("{:?}", e);
                if err.contains("Lite server error") {
                    continue;
                } else {
                    log::warn!("Failed getting state: {}", err)
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    };
    Ok(state)
}

#[allow(dead_code)]
async fn get_code_hash(node: &NodeClient, address: &MsgAddressInt) -> Result<ton_types::UInt256> {
    let state = get_account_state_with_retries(node, address).await?;
    let state = match state {
        RawContractState::NotExists => {
            anyhow::bail!("{} contracts doesn't exist", &address)
        }
        RawContractState::Exists(a) => a,
    };
    calculate_code_hash(&state.account)
}

pub fn calculate_code_hash(account: &AccountStuff) -> Result<ton_types::UInt256> {
    match &account.storage.state {
        ton_block::AccountState::AccountActive(state) => {
            let code = state
                .code
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Wallet not deployed"))?;
            Ok(code.repr_hash())
        }
        _ => anyhow::bail!("Wallet not deployed"),
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use bigdecimal::BigDecimal;
    use indexer_lib::{BounceHandler, TransactionExt};
    use log::LevelFilter;
    use node_indexer::{Config, NodeClient};
    use sqlx::PgPool;
    use ton_block::{Deserializable, MsgAddressInt, Transaction};
    use ton_token_unpacker::num_traits::ToPrimitive;

    use crate::models::owners_cache::OwnersCache;
    use crate::models::root_contracts_cache::RootContractsCache;
    use crate::sqlx_client::SqlxClient;
    use crate::ws_indexer::functions_history::{
        get_root_details, get_token_wallet_details, internal_transfer_bounced, map_burn,
        map_internal_transfer, map_mint, ParseContext,
    };
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
    async fn owner() {
        let (node, _db, owners, contracts) = init().await;
        let address = MsgAddressInt::from_str(
            "0:1abe9816647d509f59a1eadc50064c381f61633d116802e0330a18bc064d48bb",
        )
        .unwrap();

        let _res = super::get_receiver_info(address, &node, &owners, &contracts)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn owner_strange() {
        env_logger::builder()
            .filter(None, LevelFilter::Trace)
            .init();
        let (node, db, owners, contracts) = init().await;
        let tx=  "te6ccgECCQEAAg8AA7V7r4NRdje5pOIP5rogysFkYm7eDGpn57LswRqZIdCVA+AAANKSoIMIFtCZC63cUTlqP/m6mr404x3NodkhTOjagZZ9EtGuUaSwAADSkYoJ+BYK05oAABRh6EuoBQQBAhcER0kC+vCAGGHoSBEDAgBbwAAAAAAAAAAAAAAAAS1FLaRJ5QuM990nhh8UYSKv4bVGu4tw/IIW8MYUE5+OBACcJ8wMNQAAAAAAAAAAAAMAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAIJy24a47n2aeE6WaYff/GBjC7WN2eVZ6LLqdo/2o7nwo4Q+Du4QoN9OqeXvDd4UFvM528SjSfokZkieDNIhPOOwdgEBoAYBsWgBQEWHDb97JYd3TQBBaEy9QlUwej7VjAw/Jvg0FlO6Wh8ALr4NRdje5pOIP5rogysFkYm7eDGpn57LswRqZIdCVA+QL68IAAYrwzYAABpSU5ZPBMFacy7ABwHtGNIXAgAAAAAAAAAAAABa8xB6QADtKXEllen0XhWPDo3AmnPPUSX/uCDwdSXvyKmO7Dh5UIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABACgIsOG372Sw7umgCC0Jl6hKpg9H2rGBh+TfBoLKd0tD8IAAA=";
        let tx = ton_block::Transaction::construct_from_base64(tx).unwrap();
        let root_functions = prep_functions();
        let input = indexer_lib::ExtractInput {
            transaction: &tx,
            hash: tx.tx_hash().unwrap(),
            what_to_extract: &root_functions,
        };
        let res = input.process().unwrap().unwrap();
        if let Err(e) =
            super::parse_transactions_functions(res, &node, &db, &owners, &contracts, [0; 32]).await
        {
            e.chain().for_each(|cause| eprintln!("because: {}", cause))
        }
    }

    #[tokio::test]
    async fn bad_mint() {
        env_logger::builder()
            .filter(None, LevelFilter::Trace)
            .init();
        let (node, db, owners, contracts) = init().await;
        let tx=  "te6ccgECBwEAAaMAA7V9kIsUDpCgNyiz2JK0hG+TF9Fk2jBqe9NDpeMlaJEzRDAAANelduCAO60xsf2kZIAVLVOHMEHfAz7oxo3+zXuJLbR9Ihi0IS3wAADXpXbggBYLmqjAABRrp2YIBQQBAhMECOJVEBhrp2YRAwIAW8AAAAAAAAAAAAAAAAEtRS2kSeULjPfdJ4YfFGEir+G1RruLcPyCFvDGFBOfjgQAnEL7yIygAAAAAAAAAACiAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACCct0LtR/2QApol7iPZOwYF+6m7NuudQqgtwrqgrS1rnt64uWYNm00gH0v5V8OV7N74a6oX83Z/19afKb/QYRotx4BAaAGANdoAB3HJmHbttAZzmOa1Ih447INO2DaKTU32SrTo9caCdZvADZCLFA6QoDcos9iStIRvkxfRZNowanvTQ6XjJWiRM0QziVRAAYUWGAAABr0rkN5hsFzVQIFn+ergAAAAAAAAAAAAABBE3+LAEA=";
        let tx = ton_block::Transaction::construct_from_base64(tx).unwrap();
        let root_functions = prep_functions();
        let input = indexer_lib::ExtractInput {
            transaction: &tx,
            hash: tx.tx_hash().unwrap(),
            what_to_extract: &root_functions,
        };
        let res = input.process().unwrap().unwrap();
        let res = super::map_mint(
            res,
            [0; 32],
            ParseContext {
                node: &node,
                owners_cache: &owners,
                root_contracts_cache: &contracts,
            },
        )
        .await
        .unwrap();
        dbg!(res);
    }

    #[tokio::test]
    async fn bounced() {
        env_logger::builder()
            .filter(None, LevelFilter::Trace)
            .init();
        let (node, db, owners, contracts) = init().await;
        let tx=  "te6ccgECBwEAAbcAA7V+L5yrhOvwEIynP2iXeJzXNzNuPXg32o3TdwCabRlGElAAAM6/IfhcF/KJM2BNJmMjacPFk+vSSyVKy9wQ2aBUMOrGHpLQDGewAADOvxljGBYKPiSgABRq4iWIBQQBAhcMRwkFul2mGGriIhEDAgBbwAAAAAAAAAAAAAAAAS1FLaRJ5QuM990nhh8UYSKv4bVGu4tw/IIW8MYUE5+OBACeQslMF3Y8AAAAAAAAAACUAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACCclYmhaXavtui0cXF4NpYeyU2xG60GRpjdEhxvTSEDsY0oKfme9kgcV7/PTDgJ89Q2KNd3falzHJyf5axkz3Yqe0BAaAGAPlYAIZygjGHb5MY+eJ2qskZzE2hdns2otv4A1AyykjmPdFVADi+cq4Tr8BCMpz9ol3ic1zczbj14N9qN03cAmm0ZRhJUFul2mAGFFhgAAAZ1+PE+YTBR8SIf////4xpC4EAAAAAAAAAAAAAAAAAAAH0YPsaHyfqzIFhrPYcwA==";
        let tx = ton_block::Transaction::construct_from_base64(tx).unwrap();
        let root_functions = prep_functions();
        let (node, _db, owners, contracts) = init().await;
        let input = indexer_lib::ExtractInput {
            transaction: &tx,
            hash: tx.tx_hash().unwrap(),
            what_to_extract: &root_functions,
        };
        let res = input.process().unwrap().unwrap();
        dbg!(&res.output);
        let _res = internal_transfer_bounced(
            res,
            [0; 32],
            ParseContext {
                node: &node,
                owners_cache: &owners,
                root_contracts_cache: &contracts,
            },
        )
        .await
        .unwrap();
        assert_eq!(_res.amount, BigDecimal::from_str("1000").unwrap());
    }

    #[tokio::test]
    async fn test_burn() {
        let tx = "te6ccgECDgEAA1YAA7d3LK0Ml8yNhO72g37hEPj2fJ03bK5X1oQwj+MOgx33NKAAAMwljb+YGAJP+jEdP3FyiRe0hMgbPdIG6tzsQ3mIJB6zpM2tyrzAAADMC2UMIBYJ2HHAADSAKaZj6AUEAQIbBKRiCQc70mgYgCfgfxEDAgBvyZRshEw2dnwAAAAAAAQAAgAAAANvB+ce9sL+t9nVm80XJK6J+4112RcufoWnTjIO7gwN+EEQQRQAnko1bB2hEAAAAAAAAAABIQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgnJKOyGPR81I3+Ruu+d4D/WpjYBgTSb4Ro6oEj1deXt5BB+O2ztGPjDVa4LpadAecngY/FkCQ46uxoge8XmvRmVoAgHgCgYBAd8HAbFoAOWVoZL5kbCd3tBv3CIfHs+Tpu2VyvrQhhH8YdBjvuaVAAO+4rgexQs11uQJhEaqLkwYCCYDPzQY2R/uqW6gbytIUG4dsoAGNnbSAAAZhLG38wTBOw44wAgC7TzJED4AAAAAAAAAAAAAAAU1GZquAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACABfall5cer/pGqZF9mmDCYfblXNyjRl9JhiXwD8ILHTJQAl1qJJPzoRsEotyWsJDTWMSdSEovVp7ZSnPrzxm9ckTmDQkAQ4AF9qWXlx6v+kapkX2aYMJh9uVc3KNGX0mGJfAPwgsdMlABsWgBLrUSSfnQjYJRbktYSGmsYk6kJRerT2ylOfXnjN65InMAHLK0Ml8yNhO72g37hEPj2fJ03bK5X1oQwj+MOgx33NKQc70mgAY2dtIAABmEsT3hBME7DibACwHtLiiIqgAAAAAAAAAAAAAABTUZmq4AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAIAF9qWXlx6v+kapkX2aYMJh9uVc3KNGX0mGJfAPwgsdMlAAvtSy8uPV/0jVMi+zTBhMPtyrm5Roy+kwxL4B+EFjpkoMAUOAAd9xXA9ihZrrcgTCI1UXJgwEEwGfmgxsj/dUt1A3laQwDQAoippHOcDvBA/ukVXs3fZHb2qkFJU=";
        let tx = Transaction::construct_from_base64(tx).unwrap();
        let root_functions = prep_functions();
        let (node, _db, owners, contracts) = init().await;
        let input = indexer_lib::ExtractInput {
            transaction: &tx,
            hash: tx.tx_hash().unwrap(),
            what_to_extract: &root_functions,
        };
        let res = input.process().unwrap().unwrap();
        let _res = map_burn(
            res,
            [0; 32],
            ParseContext {
                node: &node,
                owners_cache: &owners,
                root_contracts_cache: &contracts,
            },
        )
        .await
        .unwrap();
    }

    fn all_functions() -> Vec<indexer_lib::FunctionOpts<BounceHandler>> {
        use super::super::abi::TON_TOKEN_WALLET;
        let contract = ton_abi::Contract::load(std::io::Cursor::new(TON_TOKEN_WALLET)).unwrap();
        contract
            .functions()
            .iter()
            .map(|x| x.1.clone().into())
            .collect()
    }

    #[tokio::test]
    async fn test_wh() {
        env_logger::builder()
            .filter(None, LevelFilter::Trace)
            .init();
        let tx = "te6ccgECCQEAAhUAA7V+L1+wCGwbH2mADm86dv/x8IbS0JofDgjjyaKxFYKa/SAAAO/es6lgEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAYPVgBwAD5gosIIAwIBAB8ECQKcXooBwDBRYQMKLDBAAIJykK7Illr6uxbrw8ubQI665xthjXh4i8gNCYQ1k8rJjaSQrsiWWvq7FuvDy5tAjrrnG2GNeHiLyA0JhDWTysmNpAIB4AYEAQHfBQD5WAHF6/YBDYNj7TABzedO3/4+ENpaE0PhwRx5NFYisFNfpQA2Mq+AxjF4u6R515b89GvtDJHMSmRqXfedv0p+Cl6UDdApiN+gBhRYYAAAHfvWdSwEwerADn////+MaQuBAAAAAAAAAAAAAAAAAAAAAIAAAAAAAAAAAAAAAEABsWgBsZV8BjGLxd0jzry356NfaGSOYlMjUu+87fpT8FL0oG8AOL1+wCGwbH2mADm86dv/x8IbS0JofDgjjyaKxFYKa/SQKcXooAYrwzYAAB371dyVhMHqv/zABwHtGNIXAgAAAAAAAAAAAAAAAAAAAAEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAIAMGccSJKj9C9ivvoVwf605RXCBvM0r71ZkSrcmjG5Lm5ABgzjiRJUfoXsV99CuD/WnKK4QN5mlferMiVbk0Y3Jc3MIAAA=";
        let tx = Transaction::construct_from_base64(tx).unwrap();
        let all_functions = prep_functions();
        let (node, _db, owners, contracts) = init().await;
        let input = indexer_lib::ExtractInput {
            transaction: &tx,
            hash: tx.tx_hash().unwrap(),
            what_to_extract: &all_functions,
        };
        let res = input.process().unwrap().unwrap();
        println!("{}", &res.output[0].function_name);
        let res = map_internal_transfer(
            res,
            [0; 32],
            ParseContext {
                node: &node,
                owners_cache: &owners,
                root_contracts_cache: &contracts,
            },
        )
        .await
        .unwrap();
        dbg!(&res);
    }

    #[tokio::test]
    async fn test_send() {
        env_logger::builder()
            .filter(None, LevelFilter::Trace)
            .init();
        let tx = "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";
        let tx = Transaction::construct_from_base64(tx).unwrap();
        let all_functions = prep_functions();
        let (node, _db, owners, contracts) = init().await;
        let input = indexer_lib::ExtractInput {
            transaction: &tx,
            hash: tx.tx_hash().unwrap(),
            what_to_extract: &all_functions,
        };
        let res = input.process().unwrap().unwrap();
        println!("{}", &res.output[0].function_name);
        let res = map_internal_transfer(
            res,
            [0; 32],
            ParseContext {
                node: &node,
                owners_cache: &owners,
                root_contracts_cache: &contracts,
            },
        )
        .await
        .unwrap();
        dbg!(&res);
    }

    #[tokio::test]
    async fn test_scale() {
        env_logger::builder()
            .filter(None, LevelFilter::Trace)
            .init();
        let tx = "te6ccgECBwEAAaMAA7V/S4Ucq/Vv1flGgglK0E53biX8v3gwC+mSkddjQpKXWgAAAPDiVBX0WlrXLQTFCy5cZLPB0gPHwjRaBuF+GkvWR7mdwTzfyWAwAADw4lQV9DYPfiHwABRrp2YIBQQBAhMECOJVEBhrp2YRAwIAW8AAAAAAAAAAAAAAAAEtRS2kSeULjPfdJ4YfFGEir+G1RruLcPyCFvDGFBOfjgQAnEL7yIygAAAAAAAAAACiAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACCcoboxLu0JOnuaTkw5iaA7i29E9YcwXzX9DdXKDvt6TIPrGwx6j1flfrTyG8x5HPbg5I2+3uooBIFXZ3BDK444ssBAaAGANdoAeHl5460FkT03iNk0/4p/Xbi1FkKFk1wBUeq1ukoZahhAD0uFHKv1b9X5RoIJStBOd24l/L94MAvpkpHXY0KSl1oDiVRAAYUWGAAAB4cSoK+hsHvxD4Fn+ergAAAAAAAAAAAAca/NJXbAEA=";
        let tx = Transaction::construct_from_base64(tx).unwrap();
        let all_functions = prep_functions();
        let (node, _db, owners, contracts) = init().await;
        let input = indexer_lib::ExtractInput {
            transaction: &tx,
            hash: tx.tx_hash().unwrap(),
            what_to_extract: &all_functions,
        };
        let res = input.process().unwrap().unwrap();
        println!("{}", &res.output[0].function_name);
        let res = map_mint(
            res,
            [0; 32],
            ParseContext {
                node: &node,
                owners_cache: &owners,
                root_contracts_cache: &contracts,
            },
        )
        .await
        .unwrap();
        dbg!(&res);
    }

    #[tokio::test]
    async fn test_bad_receiver() {
        env_logger::builder()
            .filter(None, LevelFilter::Trace)
            .init();
        let tx = "te6ccgECCQEAAg8AA7V7r4NRdje5pOIP5rogysFkYm7eDGpn57LswRqZIdCVA+AAANKSoIMIFtCZC63cUTlqP/m6mr404x3NodkhTOjagZZ9EtGuUaSwAADSkYoJ+BYK05oAABRh6EuoBQQBAhcER0kC+vCAGGHoSBEDAgBbwAAAAAAAAAAAAAAAAS1FLaRJ5QuM990nhh8UYSKv4bVGu4tw/IIW8MYUE5+OBACcJ8wMNQAAAAAAAAAAAAMAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAIJy24a47n2aeE6WaYff/GBjC7WN2eVZ6LLqdo/2o7nwo4Q+Du4QoN9OqeXvDd4UFvM528SjSfokZkieDNIhPOOwdgEBoAYBsWgBQEWHDb97JYd3TQBBaEy9QlUwej7VjAw/Jvg0FlO6Wh8ALr4NRdje5pOIP5rogysFkYm7eDGpn57LswRqZIdCVA+QL68IAAYrwzYAABpSU5ZPBMFacy7ABwHtGNIXAgAAAAAAAAAAAABa8xB6QADtKXEllen0XhWPDo3AmnPPUSX/uCDwdSXvyKmO7Dh5UIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABACgIsOG372Sw7umgCC0Jl6hKpg9H2rGBh+TfBoLKd0tD8IAAA=";
        let tx = Transaction::construct_from_base64(tx).unwrap();
        let all_functions = prep_functions();
        let (node, _db, owners, contracts) = init().await;
        let input = indexer_lib::ExtractInput {
            transaction: &tx,
            hash: tx.tx_hash().unwrap(),
            what_to_extract: &all_functions,
        };
        let res = input.process().unwrap().unwrap();
        println!("{}", &res.output[0].function_name);
        dbg!(&res.output[0].input);
        let res = map_internal_transfer(
            res,
            [0; 32],
            ParseContext {
                node: &node,
                owners_cache: &owners,
                root_contracts_cache: &contracts,
            },
        )
        .await
        .unwrap();
        dbg!(&res);
    }

    #[tokio::test]
    async fn test_bad_sender() {
        env_logger::builder()
            .filter(None, LevelFilter::Trace)
            .init();
        let tx = "te6ccgECDQEAAxsAA7dyF/bmcCyfIAkoz/EIYiUwCOtzv1U7LfzHzdaKCfLaFzAAANhm4aXAFUPjr3xbBDjSFYWUkuwzad8R1Pa0I5rXQYfrtajI9DEwAADYZtr4xBYLuE3QADSALIzKiAUEAQIZBEXJFWoTthiAKy6wEQMCAG/JkGksTCvC9AAAAAAABAACAAAAAu8QvrqrQKxuForJmQ1IvWO7PKQ1PExfOUeVaHrBzHagQNAzxACeSw4MPQkAAAAAAAAAAAF8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACCcunNCzcWj8x5nj2tp8VsS1S2b4A7Y9plgOtviDisnl6XnU60twEEmEmOnm0H3AL9ZS0x5Iu4r3ijuIKEqZfJjJsCAeAJBgEB3wcBsWgAQv7czgWT5AElGf4hDESmAR1ud+qnZb+Y+brRQT5bQucAC7W7C6tKoiM+S+khgMqvtfkLFhwj6snMsokgdMnZlwBRULgcAAYrwzYAABsM3DS4BMF3CbrACAHtGNIXAgAAAAAAAAAAAAAAAAAIWtkAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAIAbNms/jYM2OPca30HKk/A+s7x0hWXbQ07vECIAsrJe+dACOfp9NQWk7YNXhIk5HZDFE8rA/fAQgJaVtnSWs0bsYQEMAbFoAbNms/jYM2OPca30HKk/A+s7x0hWXbQ07vECIAsrJe+dAAhf25nAsnyAJKM/xCGIlMAjrc79VOy38x83Wigny2hc0VahO2AGMwFmAAAbDNvZKobBdwmowAoB6z8Q0asAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAIAAzlJhoemu0kmNh6t9Ltt3bo0KKTjOL/Zvj0xARwOXpsAAAAAAAAAAAAAAAAABC1sgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABALAUOAEc/T6agtJ2wavCRJyOyGKJ5WB++AhAS0rbOktZo3YwgIDAAA";
        let tx = Transaction::construct_from_base64(tx).unwrap();
        let all_functions = prep_functions();
        let (node, _db, owners, contracts) = init().await;
        let input = indexer_lib::ExtractInput {
            transaction: &tx,
            hash: tx.tx_hash().unwrap(),
            what_to_extract: &all_functions,
        };
        let res = input.process().unwrap().unwrap();
        println!("{}", &res.output[0].function_name);
        dbg!(&res.output[0].input);
        let _res = map_internal_transfer(
            res,
            [0; 32],
            ParseContext {
                node: &node,
                owners_cache: &owners,
                root_contracts_cache: &contracts,
            },
        )
        .await
        .unwrap()
        .unwrap();
    }
    #[tokio::test]
    async fn test_bad_mint() {
        env_logger::builder()
            .filter(None, LevelFilter::Trace)
            .init();
        let tx = "te6ccgECBwEAAaMAA7V11ATlP/zgRIElxhlMISvql4lb5J9x9tUAbO37sEJGSHAAANhoMGMcMi5w/AdxLnbjY2Es00K9V+gbTGg6PoZ0OrGHzSe/aZCQAADYaDBjHBYLuIEwABRrp2YIBQQBAhMECOJVEBhrp2YRAwIAW8AAAAAAAAAAAAAAAAEtRS2kSeULjPfdJ4YfFGEir+G1RruLcPyCFvDGFBOfjgQAnEL7yIygAAAAAAAAAACiAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACCckhBAtI0VCestFGwS5nLa9Wj6E1suNbyu4AgVz4oHSFk2E/VS4Ri0iUEKtdzr+wMIQnmJ/IkYfbtRWxpeeS3FdIBAaAGANdoAB3HJmHbttAZzmOa1Ih447INO2DaKTU32SrTo9caCdZvABdQE5T/84ESBJcYZTCEr6peJW+SfcfbVAGzt+7BCRkhziVRAAYUWGAAABsNBZJRhsF3EA4Fn+ergAAAAAAAAAAAAAAAAJVB4EA=";
        let tx = Transaction::construct_from_base64(tx).unwrap();
        let all_functions = prep_functions();
        let (node, _db, owners, contracts) = init().await;
        let input = indexer_lib::ExtractInput {
            transaction: &tx,
            hash: tx.tx_hash().unwrap(),
            what_to_extract: &all_functions,
        };
        let res = input.process().unwrap().unwrap();
        println!("{}", &res.output[0].function_name);
        dbg!(&res.output[0].input);
        let res = map_mint(
            res,
            [0; 32],
            ParseContext {
                node: &node,
                owners_cache: &owners,
                root_contracts_cache: &contracts,
            },
        )
        .await
        .unwrap();
        dbg!(res);
    }

    #[tokio::test]
    async fn test_bad_burn() {
        env_logger::builder()
            .filter(None, LevelFilter::Trace)
            .init();
        let tx = "te6ccgECDgEAA0EAA7d0oWcoKAyzBdyKoinglC8NKBoFdJn8nwppY3aqXgdRp0AAAM6+MaTsGkkv2cL3+PtmGXh8Ftl9RmgOitpNvRXMy3jYIvYudM5QAADOvhUItBYKPf/wADSAKXFzqAUEAQIZBGqJFlBxvhiAJ9e1EQMCAG/JkzQETDM1LAAAAAAABAACAAAAAlm+NweSpGSDJQFWyCtTPUy/fQQ4Mn66x1RaYuK7YaKqQRA8FACeSjMsPQkAAAAAAAAAAAEgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACCcjUDLsylqidgrcrOrVa80OKrSREd96ujGAt9BZnfMAIh/bZStpr4QJ9NWsgjJjbY+xdo7Sevb7CP6D+PWJ2WaZECAeAKBgEB3wcBsWgAlCzlBQGWYLuRVEU8EoXhpQNArpM/k+FNLG7VS8DqNOkALUP6uhoPt7d/nojf+/vm1qLmWzXcJD+jHUmpe/E9kMsRX3KFIAYzNXoAABnXxjSdhMFHv/7ACALtPMkQPgAAAAAAAAAAAAAAAAAAE4jB9jQ+T9WZAsNZ7DmqRUTz2MgvJ//s2Wcokw7XpCwlyoAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABADi+cq4Tr8BCMpz9ol3ic1zczbj14N9qN03cAmm0ZRhJYNCQBDgBxfOVcJ1+AhGU5+0S7xOa5uZtx68G+1G6buATTaMowksAGxaAHF85VwnX4CEZTn7RLvE5rm5m3Hrwb7Ubpu4BNNoyjCSwAShZygoDLMF3IqiKeCULw0oGgV0mfyfCmljdqpeB1GnRFlBxvgBjM1egAAGdfFnAcEwUe/6sALAe0uKIiqAAAAAAAAAAAAAAAAAAATiMH2ND5P1ZkCw1nsOapFRPPYyC8n/+zZZyiTDtekLCXKgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEAOL5yrhOvwEIynP2iXeJzXNzNuPXg32o3TdwCabRlGElgwBQ4AWof1dDQfb27/PRG/9/fNrUXMtmu4SH9GOpNS9+J7IZZANAAA=";
        let tx = Transaction::construct_from_base64(tx).unwrap();
        let all_functions = prep_functions();
        let (node, _db, owners, contracts) = init().await;
        let input = indexer_lib::ExtractInput {
            transaction: &tx,
            hash: tx.tx_hash().unwrap(),
            what_to_extract: &all_functions,
        };
        let res = input.process().unwrap().unwrap();
        println!("{}", &res.output[0].function_name);
        dbg!(&res.output[0].input);
        let res = map_burn(
            res,
            [0; 32],
            ParseContext {
                node: &node,
                owners_cache: &owners,
                root_contracts_cache: &contracts,
            },
        )
        .await
        .unwrap();
        dbg!(res);
    }

    #[tokio::test]
    async fn test_all_txs() {
        env_logger::builder()
            .filter(Some("ton"), LevelFilter::Trace)
            .init();
        let (node, db, owners, contracts) = init().await;
        let root_functions = prep_functions();

        let address = MsgAddressInt::from_str(
            "0:747cd6294c8b5642f03f1405177969beafd8effdc1983e242dba285d805bfbb9",
        )
        .unwrap();
        let all = node.get_all_transactions(address).await.unwrap();
        println!("{}", all.len());
        for tx in all {
            let input = indexer_lib::ExtractInput {
                transaction: &tx.data,
                hash: tx.hash,
                what_to_extract: &root_functions,
            };
            let res = input.process().unwrap();
            if let Some(a) = res {
                println!("{}", hex::encode(&a.transaction.tx_hash().unwrap()));
                println!("{}", &a.output[0].function_name);
                if let Err(e) =
                    super::parse_transactions_functions(a, &node, &db, &owners, &contracts, [0; 32])
                        .await
                {
                    e.chain().for_each(|cause| eprintln!("because: {}", cause))
                }
            };
        }
    }

    #[tokio::test]
    async fn get_balance() {
        use num_bigint::BigInt;
        let (node, _db, _owners, _contracts) = init().await;
        let address = MsgAddressInt::from_str(
            "0:747cd6294c8b5642f03f1405177969beafd8effdc1983e242dba285d805bfbb9",
        )
        .unwrap();
        let res = get_token_wallet_details(&node, address).await.unwrap();
        let bal = res.0.balance.to_u128().unwrap();
        let root_det = get_root_details(&node, res.0.root_address).await.unwrap();
        let tot = BigDecimal::new(BigInt::from(bal), root_det.2.decimals as i64);
        println!("{}", tot);
    }
}
