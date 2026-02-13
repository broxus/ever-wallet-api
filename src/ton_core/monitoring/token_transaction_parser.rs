use anyhow::Result;
use bigdecimal::BigDecimal;
use tycho_types::cell::Cell;
use uuid::Uuid;
use nekoton_core::contracts::blockchain_context::BlockchainContextBuilder;

use crate::ton_core::*;
use crate::utils::token_wallets::models::{TokenIncomingTransfer, TokenWalletTransaction};

struct ParseContext<'a> {
    sqlx_client: &'a SqlxClient,
    owners_cache: &'a OwnersCache,
}

pub async fn parse_token_transaction(
    token_transaction_ctx: TokenTransactionContext,
    parsed_token_transaction: TokenWalletTransaction,
    sqlx_client: &SqlxClient,
    owners_cache: &OwnersCache,
) -> Result<CreateTokenTransaction> {
    let parse_ctx = ParseContext {
        sqlx_client,
        owners_cache,
    };

    let parsed = match parsed_token_transaction {
        TokenWalletTransaction::IncomingTransfer(transfer) => {
            internal_transfer_receive(token_transaction_ctx, transfer, parse_ctx).await?
        }
        TokenWalletTransaction::Accept(tokens) => {
            internal_transfer_mint(token_transaction_ctx, tokens, parse_ctx).await?
        }
        TokenWalletTransaction::OutgoingTransfer(token_transfer) => {
            internal_transfer_send(
                token_transaction_ctx,
                token_transfer.tokens,
                Some(token_transfer.payload),
                parse_ctx,
            )
            .await?
        }
        TokenWalletTransaction::SwapBack(swap_back) => {
            internal_transfer_send(token_transaction_ctx, swap_back.tokens, None, parse_ctx).await?
        }
        TokenWalletTransaction::TransferBounced(tokens)
        | TokenWalletTransaction::SwapBackBounced(tokens) => {
            internal_transfer_bounced(token_transaction_ctx, tokens, parse_ctx).await?
        }
    };

    Ok(parsed)
}

async fn internal_transfer_send(
    token_transaction_ctx: TokenTransactionContext,
    tokens: u128,
    payload_cell: Option<Cell>,
    parse_ctx: ParseContext<'_>,
) -> Result<CreateTokenTransaction> {
    let address = StdAddr::new(0, token_transaction_ctx.account);

    let context = BlockchainContextBuilder::new().build()?;
    let owner_info =
        get_token_wallet_info(&address, &parse_ctx, token_transaction_ctx.token_state, context).await?;

    let mut message_hash = Default::default();
    for message in token_transaction_ctx.transaction.iter_out_msgs() {
        let message = message?;
        let cell_builder = CellBuilder::build_from(&message)?;
        message_hash = cell_builder.repr_hash().to_string();
    }

    let in_message_hash = token_transaction_ctx
        .transaction
        .in_msg
        .as_ref()
        .map(|message| message.repr_hash().to_string())
        .unwrap_or_default();

    let transaction = CreateTokenTransaction {
        id: Uuid::new_v4(),
        transaction_hash: Some(token_transaction_ctx.transaction_hash.to_string()),
        transaction_timestamp: token_transaction_ctx.block_utime,
        message_hash,
        owner_message_hash: None,
        account_workchain_id: owner_info.owner_address.workchain as i32,
        account_hex: owner_info.owner_address.address.to_string(),
        sender_workchain_id: None,
        sender_hex: None,
        root_address: owner_info.root_address.to_string(),
        value: -BigDecimal::new(tokens.into(), 0),
        payload: payload_cell.map(|c| Boc::encode(c)),
        block_hash: token_transaction_ctx.block_hash.to_string(),
        block_time: token_transaction_ctx.block_utime as i32,
        direction: TonTransactionDirection::Send,
        status: TonTokenTransactionStatus::Done,
        error: None,
        in_message_hash: Some(in_message_hash),
    };

    Ok(transaction)
}

async fn internal_transfer_receive(
    token_transaction_ctx: TokenTransactionContext,
    token_transfer: TokenIncomingTransfer,
    parse_ctx: ParseContext<'_>,
) -> Result<CreateTokenTransaction> {
    let address = StdAddr::new(0, token_transaction_ctx.account);
    let context = BlockchainContextBuilder::new().build()?;

    let owner_info =
        get_token_wallet_info(&address, &parse_ctx, token_transaction_ctx.token_state, context).await?;

    let message_hash = token_transaction_ctx
        .transaction
        .in_msg
        .clone()
        .map(|message| message.repr_hash().to_string())
        .unwrap_or_default();

    let payload = CellBuilder::build_from(token_transaction_ctx.in_msg)?;

    let transaction = CreateTokenTransaction {
        id: Uuid::new_v4(),
        transaction_hash: Some(token_transaction_ctx.transaction_hash.to_string()),
        transaction_timestamp: token_transaction_ctx.block_utime,
        message_hash,
        owner_message_hash: None,
        account_workchain_id: owner_info.owner_address.workchain as i32,
        account_hex: owner_info.owner_address.address.to_string(),
        sender_workchain_id: Some(token_transfer.sender_address.workchain as i32),
        sender_hex: Some(token_transfer.sender_address.address.to_string()),
        value: BigDecimal::new(token_transfer.tokens.into(), 0),
        root_address: owner_info.root_address.to_string(),
        payload: Some(Boc::encode(payload)),
        error: None,
        block_hash: token_transaction_ctx.block_hash.to_string(),
        block_time: token_transaction_ctx.block_utime as i32,
        direction: TonTransactionDirection::Receive,
        status: TonTokenTransactionStatus::Done,
        in_message_hash: None,
    };

    Ok(transaction)
}

async fn internal_transfer_bounced(
    token_transaction_ctx: TokenTransactionContext,
    tokens: u128,
    parse_ctx: ParseContext<'_>,
) -> Result<CreateTokenTransaction> {
    let address = StdAddr::new(0, token_transaction_ctx.account);
    
    let context = BlockchainContextBuilder::new().build()?;

    let owner_info =
        get_token_wallet_info(&address, &parse_ctx, token_transaction_ctx.token_state, context).await?;

    let message_hash = token_transaction_ctx
        .transaction
        .in_msg
        .clone()
        .map(|message| message.repr_hash().to_string())
        .unwrap_or_default();

    let transaction = CreateTokenTransaction {
        id: Uuid::new_v4(),
        transaction_hash: Some(token_transaction_ctx.transaction_hash.to_string()),
        transaction_timestamp: token_transaction_ctx.block_utime,
        message_hash,
        owner_message_hash: None,
        account_workchain_id: owner_info.owner_address.workchain as i32,
        account_hex: owner_info.owner_address.address.to_string(),
        sender_workchain_id: None,
        sender_hex: None,
        root_address: owner_info.root_address.to_string(),
        value: BigDecimal::new(tokens.into(), 0),
        payload: None,
        block_hash: token_transaction_ctx.block_hash.to_string(),
        block_time: token_transaction_ctx.block_utime as i32,
        direction: TonTransactionDirection::Send,
        status: TonTokenTransactionStatus::Done,
        error: None,
        in_message_hash: None,
    };

    Ok(transaction)
}

async fn internal_transfer_mint(
    token_transaction_ctx: TokenTransactionContext,
    tokens: u128,
    parse_ctx: ParseContext<'_>,
) -> Result<CreateTokenTransaction> {
    let address = StdAddr::new(0, token_transaction_ctx.account);

    let context = BlockchainContextBuilder::new().build()?;
    let owner_info =
        get_token_wallet_info(&address, &parse_ctx, token_transaction_ctx.token_state, context).await?;

    let message_hash = token_transaction_ctx
        .transaction
        .in_msg
        .clone()
        .map(|message| *message.repr_hash())
        .unwrap_or_default();

    let transaction = CreateTokenTransaction {
        id: Uuid::new_v4(),
        transaction_hash: Some(token_transaction_ctx.transaction_hash.to_string()),
        transaction_timestamp: token_transaction_ctx.block_utime,
        message_hash: message_hash.to_string(),
        owner_message_hash: None,
        account_workchain_id: owner_info.owner_address.workchain as i32,
        account_hex: owner_info.owner_address.address.to_string(),
        sender_workchain_id: None,
        sender_hex: None,
        value: BigDecimal::new(tokens.into(), 0),
        root_address: owner_info.root_address.to_string(),
        payload: None,
        error: None,
        block_hash: token_transaction_ctx.block_hash.to_string(),
        block_time: token_transaction_ctx.block_utime as i32,
        direction: TonTransactionDirection::Receive,
        status: TonTokenTransactionStatus::Done,
        in_message_hash: None,
    };

    Ok(transaction)
}

async fn get_token_wallet_info(
    contract_address: &StdAddr,
    parse_ctx: &ParseContext<'_>,
    contract: ExistingContract,
    context: BlockchainContext,
) -> Result<OwnerInfo> {
    let res = match parse_ctx.owners_cache.get(contract_address).await {
        None => {
            let (wallet, version, hash) = get_token_wallet_details(contract, context)?;
            let info = OwnerInfo {
                owner_address: wallet.owner_address,
                root_address: wallet.root_address,
                code_hash: hash,
                version,
            };

            let _check_root_address = parse_ctx
                .sqlx_client
                .get_root_token(&info.root_address.to_string())
                .await
                .map_err(|_| TonCoreError::InvalidRootToken(info.root_address.to_string()))?;

            parse_ctx
                .owners_cache
                .insert(contract_address.clone(), info.clone())
                .await;
            info
        }
        Some(a) => a,
    };
    Ok(res)
}
