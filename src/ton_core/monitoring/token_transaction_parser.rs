use anyhow::Result;
use bigdecimal::BigDecimal;
use nekoton::core::models::{TokenIncomingTransfer, TokenWalletTransaction};
use num_bigint::BigUint;
use ton_block::MsgAddressInt;
use ton_types::AccountId;
use uuid::Uuid;

use crate::ton_core::*;

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
            internal_transfer_send(token_transaction_ctx, token_transfer.tokens, parse_ctx).await?
        }
        TokenWalletTransaction::SwapBack(token_transfer) => {
            internal_transfer_send(token_transaction_ctx, token_transfer.tokens, parse_ctx).await?
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
    tokens: BigUint,
    parse_ctx: ParseContext<'_>,
) -> Result<CreateTokenTransaction> {
    let address = MsgAddressInt::with_standart(
        None,
        ton_block::BASE_WORKCHAIN_ID as i8,
        AccountId::from(token_transaction_ctx.account),
    )?;

    let owner_info =
        get_token_wallet_info(&address, &parse_ctx, &token_transaction_ctx.token_state).await?;

    let mut message_hash = Default::default();
    let _ = token_transaction_ctx
        .transaction
        .out_msgs
        .iterate(|ton_block::InRefValue(message)| {
            message_hash = message.hash().unwrap_or_default().to_hex_string();
            Ok(false)
        });

    let in_message_hash = token_transaction_ctx
        .transaction
        .in_msg
        .clone()
        .map(|message| message.hash().to_hex_string())
        .unwrap_or_default();

    let in_message_src = match token_transaction_ctx
        .transaction
        .in_msg
        .clone()
        .map(|message| message.read_struct())
    {
        Some(Ok(message)) => message.src().map(|addr| addr.address().to_hex_string()),
        _ => None,
    };

    let transaction = CreateTokenTransaction {
        id: Uuid::new_v4(),
        transaction_hash: Some(token_transaction_ctx.transaction_hash.to_hex_string()),
        transaction_timestamp: token_transaction_ctx.block_utime,
        message_hash,
        owner_message_hash: None,
        account_workchain_id: owner_info.owner_address.workchain_id(),
        account_hex: owner_info.owner_address.address().to_hex_string(),
        sender_workchain_id: None,
        sender_hex: None,
        root_address: owner_info.root_address.to_string(),
        value: -BigDecimal::new(tokens.into(), 0),
        payload: None,
        block_hash: token_transaction_ctx.block_hash.to_hex_string(),
        block_time: token_transaction_ctx.block_utime as i32,
        direction: TonTransactionDirection::Send,
        status: TonTokenTransactionStatus::Done,
        error: None,
        in_message_hash: Some(in_message_hash),
        in_message_src,
    };

    Ok(transaction)
}

async fn internal_transfer_receive(
    token_transaction_ctx: TokenTransactionContext,
    token_transfer: TokenIncomingTransfer,
    parse_ctx: ParseContext<'_>,
) -> Result<CreateTokenTransaction> {
    let address = MsgAddressInt::with_standart(
        None,
        ton_block::BASE_WORKCHAIN_ID as i8,
        AccountId::from(token_transaction_ctx.account),
    )?;

    let owner_info =
        get_token_wallet_info(&address, &parse_ctx, &token_transaction_ctx.token_state).await?;

    let message_hash = token_transaction_ctx
        .transaction
        .in_msg
        .clone()
        .map(|message| message.hash().to_hex_string())
        .unwrap_or_default();

    let transaction = CreateTokenTransaction {
        id: Uuid::new_v4(),
        transaction_hash: Some(token_transaction_ctx.transaction_hash.to_hex_string()),
        transaction_timestamp: token_transaction_ctx.block_utime,
        message_hash,
        owner_message_hash: None,
        account_workchain_id: owner_info.owner_address.get_workchain_id(),
        account_hex: owner_info.owner_address.address().to_hex_string(),
        sender_workchain_id: Some(token_transfer.sender_address.workchain_id()),
        sender_hex: Some(token_transfer.sender_address.address().to_hex_string()),
        value: BigDecimal::new(token_transfer.tokens.into(), 0),
        root_address: owner_info.root_address.to_string(),
        payload: None,
        error: None,
        block_hash: token_transaction_ctx.block_hash.to_hex_string(),
        block_time: token_transaction_ctx.block_utime as i32,
        direction: TonTransactionDirection::Receive,
        status: TonTokenTransactionStatus::Done,
        in_message_hash: None,
        in_message_src: None,
    };

    Ok(transaction)
}

async fn internal_transfer_bounced(
    token_transaction_ctx: TokenTransactionContext,
    tokens: BigUint,
    parse_ctx: ParseContext<'_>,
) -> Result<CreateTokenTransaction> {
    let address = MsgAddressInt::with_standart(
        None,
        ton_block::BASE_WORKCHAIN_ID as i8,
        AccountId::from(token_transaction_ctx.account),
    )?;

    let owner_info =
        get_token_wallet_info(&address, &parse_ctx, &token_transaction_ctx.token_state).await?;

    let message_hash = token_transaction_ctx
        .transaction
        .in_msg
        .clone()
        .map(|message| message.hash().to_hex_string())
        .unwrap_or_default();

    let transaction = CreateTokenTransaction {
        id: Uuid::new_v4(),
        transaction_hash: Some(token_transaction_ctx.transaction_hash.to_hex_string()),
        transaction_timestamp: token_transaction_ctx.block_utime,
        message_hash,
        owner_message_hash: None,
        account_workchain_id: owner_info.owner_address.workchain_id(),
        account_hex: owner_info.owner_address.address().to_hex_string(),
        sender_workchain_id: None,
        sender_hex: None,
        root_address: owner_info.root_address.to_string(),
        value: BigDecimal::new(tokens.into(), 0),
        payload: None,
        block_hash: token_transaction_ctx.block_hash.to_hex_string(),
        block_time: token_transaction_ctx.block_utime as i32,
        direction: TonTransactionDirection::Send,
        status: TonTokenTransactionStatus::Done,
        error: None,
        in_message_hash: None,
        in_message_src: None,
    };

    Ok(transaction)
}

async fn internal_transfer_mint(
    token_transaction_ctx: TokenTransactionContext,
    tokens: BigUint,
    parse_ctx: ParseContext<'_>,
) -> Result<CreateTokenTransaction> {
    let address = MsgAddressInt::with_standart(
        None,
        ton_block::BASE_WORKCHAIN_ID as i8,
        AccountId::from(token_transaction_ctx.account),
    )?;

    let owner_info =
        get_token_wallet_info(&address, &parse_ctx, &token_transaction_ctx.token_state).await?;

    let message_hash = token_transaction_ctx
        .transaction
        .in_msg
        .clone()
        .map(|message| message.hash())
        .unwrap_or_default();

    let transaction = CreateTokenTransaction {
        id: Uuid::new_v4(),
        transaction_hash: Some(token_transaction_ctx.transaction_hash.to_hex_string()),
        transaction_timestamp: token_transaction_ctx.block_utime,
        message_hash: message_hash.to_hex_string(),
        owner_message_hash: None,
        account_workchain_id: owner_info.owner_address.get_workchain_id(),
        account_hex: owner_info.owner_address.address().to_hex_string(),
        sender_workchain_id: None,
        sender_hex: None,
        value: BigDecimal::new(tokens.into(), 0),
        root_address: owner_info.root_address.to_string(),
        payload: None,
        error: None,
        block_hash: token_transaction_ctx.block_hash.to_hex_string(),
        block_time: token_transaction_ctx.block_utime as i32,
        direction: TonTransactionDirection::Receive,
        status: TonTokenTransactionStatus::Done,
        in_message_hash: None,
        in_message_src: None,
    };

    Ok(transaction)
}

async fn get_token_wallet_info(
    contract_address: &MsgAddressInt,
    parse_ctx: &ParseContext<'_>,
    contract: &ExistingContract,
) -> Result<OwnerInfo> {
    let res = match parse_ctx.owners_cache.get(contract_address).await {
        None => {
            let (wallet, version, hash) = get_token_wallet_details(contract)?;
            let info = OwnerInfo {
                owner_address: wallet.owner_address,
                root_address: wallet.root_address,
                code_hash: hash.to_vec(),
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
