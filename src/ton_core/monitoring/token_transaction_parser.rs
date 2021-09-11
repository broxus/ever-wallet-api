use anyhow::Result;
use bigdecimal::BigDecimal;
use nekoton::core::models::{TokenIncomingTransfer, TokenWalletDetails, TokenWalletTransaction};
use num_bigint::BigUint;
use ton_block::MsgAddressInt;
use ton_types::{AccountId, UInt256};
use uuid::Uuid;

use crate::ton_core::*;

const TOKEN_WALLET_CODE_HASH: [u8; 32] = [
    44, 127, 188, 81, 97, 200, 223, 145, 75, 25, 193, 126, 27, 104, 81, 113, 32, 159, 175, 201, 32,
    0, 153, 178, 193, 252, 136, 125, 89, 93, 42, 227,
];

pub async fn parse_token_transaction(
    token_transaction_ctx: TokenTransactionContext,
    parsed_token_transaction: TokenWalletTransaction,
    owners_cache: &OwnersCache,
) -> Result<CreateTokenTransaction> {
    let parse_ctx = ParseContext { owners_cache };

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

struct ParseContext<'a> {
    owners_cache: &'a OwnersCache,
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

    let owner_info = get_token_wallet_info(
        &address,
        &token_transaction_ctx.shard_accounts,
        parse_ctx.owners_cache,
    )
    .await?;

    let mut message_hash = Default::default();
    let _ = token_transaction_ctx
        .transaction
        .out_msgs
        .iterate(|ton_block::InRefValue(message)| {
            message_hash = message.hash().unwrap_or_default();
            Ok(false)
        });

    let owner_message_hash = token_transaction_ctx
        .transaction
        .in_msg
        .clone()
        .map(|message| message.hash())
        .unwrap_or_default();

    let mut transaction = CreateTokenTransaction {
        id: Uuid::new_v4(),
        transaction_hash: Some(token_transaction_ctx.transaction_hash.to_hex_string()),
        message_hash: message_hash.to_hex_string(),
        owner_message_hash: Some(owner_message_hash.to_hex_string()),
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
    };

    if TOKEN_WALLET_CODE_HASH.as_ref() != owner_info.code_hash {
        transaction.error = Some("Bad hash".to_string())
    }

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

    let owner_info = get_token_wallet_info(
        &address,
        &token_transaction_ctx.shard_accounts,
        parse_ctx.owners_cache,
    )
    .await?;

    let message_hash = token_transaction_ctx
        .transaction
        .in_msg
        .clone()
        .map(|message| message.hash())
        .unwrap_or_default();

    let mut transaction = CreateTokenTransaction {
        id: Uuid::new_v4(),
        transaction_hash: Some(token_transaction_ctx.transaction_hash.to_hex_string()),
        message_hash: message_hash.to_hex_string(),
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
    };
    if TOKEN_WALLET_CODE_HASH.as_ref() != owner_info.code_hash {
        transaction.error = Some("Bad hash".to_string())
    }

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

    let owner_info = get_token_wallet_info(
        &address,
        &token_transaction_ctx.shard_accounts,
        parse_ctx.owners_cache,
    )
    .await?;

    let message_hash = token_transaction_ctx
        .transaction
        .in_msg
        .clone()
        .map(|message| message.hash())
        .unwrap_or_default();

    let mut transaction = CreateTokenTransaction {
        id: Uuid::new_v4(),
        transaction_hash: Some(token_transaction_ctx.transaction_hash.to_hex_string()),
        message_hash: message_hash.to_hex_string(),
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
    };
    if TOKEN_WALLET_CODE_HASH.as_ref() != owner_info.code_hash {
        transaction.error = Some("Bad hash".to_string())
    }

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

    let owner_info = get_token_wallet_info(
        &address,
        &token_transaction_ctx.shard_accounts,
        parse_ctx.owners_cache,
    )
    .await?;

    let message_hash = token_transaction_ctx
        .transaction
        .in_msg
        .clone()
        .map(|message| message.hash())
        .unwrap_or_default();

    let mut transaction = CreateTokenTransaction {
        id: Uuid::new_v4(),
        transaction_hash: Some(token_transaction_ctx.transaction_hash.to_hex_string()),
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
    };
    if TOKEN_WALLET_CODE_HASH.as_ref() != owner_info.code_hash {
        transaction.error = Some("Bad hash".to_string())
    }

    Ok(transaction)
}

async fn get_token_wallet_info(
    contract_address: &MsgAddressInt,
    shard_accounts: &ton_block::ShardAccounts,
    owners_cache: &OwnersCache,
) -> Result<OwnerInfo> {
    let res = match owners_cache.get(contract_address).await {
        None => {
            let (wallet, hash) = get_token_wallet_details(contract_address, shard_accounts).await?;
            let info = OwnerInfo {
                owner_address: wallet.owner_address,
                root_address: wallet.root_address,
                code_hash: hash.to_vec(),
            };
            owners_cache
                .insert(contract_address.clone(), info.clone())
                .await;
            info
        }
        Some(a) => a,
    };
    Ok(res)
}

async fn get_token_wallet_details(
    address: &MsgAddressInt,
    shard_accounts: &ton_block::ShardAccounts,
) -> Result<(TokenWalletDetails, [u8; 32])> {
    let account = UInt256::from_be_bytes(&address.address().get_bytestring(0));
    let state = shard_accounts
        .find_account(&account)?
        .ok_or_else(|| TonCoreError::AccountNotExist(account.to_hex_string()))?;

    let state = nekoton::core::token_wallet::TokenWalletContractState(&state);
    let hash = *state.get_code_hash()?.as_slice();
    let version = state.get_version()?;
    let details = state.get_details(version)?;
    Ok((details, hash))
}
