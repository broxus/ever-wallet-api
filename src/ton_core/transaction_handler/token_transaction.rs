use std::str::FromStr;

use anyhow::Result;
use bigdecimal::BigDecimal;
use nekoton::core::models::{TokenIncomingTransfer, TokenWalletDetails, TokenWalletTransaction};
use num_bigint::BigUint;
use ton_block::MsgAddressInt;
use ton_types::{AccountId, UInt256};
use uuid::Uuid;

use crate::models::account_enums::{TonTokenTransactionStatus, TonTransactionDirection};
use crate::models::owners_cache::{OwnerInfo, OwnersCache};
use crate::ton_core::*;

pub const TOKEN_WALLET_CODE_HASH: [u8; 32] = [
    44, 127, 188, 81, 97, 200, 223, 145, 75, 25, 193, 126, 27, 104, 81, 113, 32, 159, 175, 201, 32,
    0, 153, 178, 193, 252, 136, 125, 89, 93, 42, 227,
];

pub async fn handle_token_transaction(
    token_transaction_ctx: TokenTransactionContext,
    parsed_token_transaction: TokenWalletTransaction,
    owners_cache: &OwnersCache,
) -> Result<ReceiveTokenTransaction> {
    let parse_ctx = ParseContext { owners_cache };

    let parsed = match parsed_token_transaction {
        TokenWalletTransaction::IncomingTransfer(transfer) => {
            internal_transfer_receive(token_transaction_ctx, transfer, parse_ctx).await?
        }
        TokenWalletTransaction::Accept(tokens) => {
            internal_transfer_mint(token_transaction_ctx, tokens, parse_ctx).await?
        }
        TokenWalletTransaction::OutgoingTransfer(_) | TokenWalletTransaction::SwapBack(_) => {
            internal_transfer_send(token_transaction_ctx, parse_ctx).await?
        }
        TokenWalletTransaction::TransferBounced(_) | TokenWalletTransaction::SwapBackBounced(_) => {
            internal_transfer_bounced(token_transaction_ctx, parse_ctx).await?
        }
    };

    Ok(parsed)
}

struct ParseContext<'a> {
    owners_cache: &'a OwnersCache,
}

async fn internal_transfer_send(
    token_transaction_ctx: TokenTransactionContext,
    parse_ctx: ParseContext<'_>,
) -> Result<ReceiveTokenTransaction> {
    let sender_address = MsgAddressInt::with_standart(
        None,
        ton_block::BASE_WORKCHAIN_ID as i8,
        AccountId::from(token_transaction_ctx.account),
    )?;

    let sender_info = get_token_wallet_info(
        &sender_address,
        &token_transaction_ctx.shard_accounts,
        parse_ctx.owners_cache,
    )
    .await?;

    let mut transaction = UpdateSentTokenTransaction {
        message_hash: token_transaction_ctx.message_hash.to_hex_string(),
        account_workchain_id: sender_address.workchain_id(),
        account_hex: sender_address.address().to_hex_string(),
        root_address: sender_info.root_address.to_string(),
        input: UpdateSendTokenTransaction {
            transaction_hash: Some(token_transaction_ctx.transaction_hash.to_hex_string()),
            payload: None,
            block_hash: Some(token_transaction_ctx.block_hash.to_hex_string()),
            block_time: Some(token_transaction_ctx.block_utime as i32),
            status: TonTokenTransactionStatus::Done,
            error: None,
        },
    };

    if TOKEN_WALLET_CODE_HASH.as_ref() != sender_info.code_hash {
        transaction.input.error = Some("Bad hash".to_string())
    }

    Ok(ReceiveTokenTransaction::UpdateSent(transaction))
}

async fn internal_transfer_receive(
    token_transaction_ctx: TokenTransactionContext,
    token_transfer: TokenIncomingTransfer,
    parse_ctx: ParseContext<'_>,
) -> Result<ReceiveTokenTransaction> {
    let receiver_address = MsgAddressInt::with_standart(
        None,
        ton_block::BASE_WORKCHAIN_ID as i8,
        AccountId::from(token_transaction_ctx.account),
    )?;

    let receiver_info = get_token_wallet_info(
        &receiver_address,
        &token_transaction_ctx.shard_accounts,
        parse_ctx.owners_cache,
    )
    .await?;

    let amount = BigDecimal::new(token_transfer.tokens.into(), 0);

    let mut transaction = CreateReceiveTokenTransaction {
        id: Uuid::new_v4(),
        transaction_hash: Some(token_transaction_ctx.transaction_hash.to_hex_string()),
        message_hash: token_transaction_ctx.message_hash.to_hex_string(),
        account_workchain_id: receiver_address.get_workchain_id(),
        account_hex: receiver_address.address().to_hex_string(),
        sender_workchain_id: Some(token_transfer.sender_address.workchain_id()),
        sender_hex: Some(token_transfer.sender_address.address().to_hex_string()),
        value: amount,
        root_address: receiver_info.root_address.to_string(),
        payload: None,
        error: None,
        block_hash: token_transaction_ctx.block_hash.to_hex_string(),
        block_time: token_transaction_ctx.block_utime as i32,
        direction: TonTransactionDirection::Receive,
        status: TonTokenTransactionStatus::New,
    };

    if TOKEN_WALLET_CODE_HASH.as_ref() != receiver_info.code_hash {
        transaction.error = Some("Bad hash".to_string())
    }

    Ok(ReceiveTokenTransaction::Create(transaction))
}

async fn internal_transfer_bounced(
    token_transaction_ctx: TokenTransactionContext,
    parse_ctx: ParseContext<'_>,
) -> Result<ReceiveTokenTransaction> {
    let sender_address = MsgAddressInt::with_standart(
        None,
        ton_block::BASE_WORKCHAIN_ID as i8,
        AccountId::from(token_transaction_ctx.account),
    )?;

    let sender_info = get_token_wallet_info(
        &sender_address,
        &token_transaction_ctx.shard_accounts,
        parse_ctx.owners_cache,
    )
    .await?;

    let mut transaction = UpdateSentTokenTransaction {
        message_hash: "".to_string(),
        account_workchain_id: sender_address.workchain_id(),
        account_hex: sender_address.address().to_hex_string(),
        root_address: sender_info.root_address.to_string(),
        input: UpdateSendTokenTransaction {
            transaction_hash: Some(token_transaction_ctx.transaction_hash.to_hex_string()),
            payload: None,
            block_hash: Some(token_transaction_ctx.block_hash.to_hex_string()),
            block_time: Some(token_transaction_ctx.block_utime as i32),
            status: TonTokenTransactionStatus::Error,
            error: None,
        },
    };

    if TOKEN_WALLET_CODE_HASH.as_ref() != sender_info.code_hash {
        transaction.input.error = Some("Bad hash".to_string())
    }

    Ok(ReceiveTokenTransaction::UpdateSent(transaction))
}

async fn internal_transfer_mint(
    token_transaction_ctx: TokenTransactionContext,
    tokens: BigUint,
    parse_ctx: ParseContext<'_>,
) -> Result<ReceiveTokenTransaction> {
    let receiver_address = MsgAddressInt::with_standart(
        None,
        ton_block::BASE_WORKCHAIN_ID as i8,
        AccountId::from(token_transaction_ctx.account),
    )?;

    let receiver_info = get_token_wallet_info(
        &receiver_address,
        &token_transaction_ctx.shard_accounts,
        parse_ctx.owners_cache,
    )
    .await?;

    let amount = BigDecimal::new(tokens.into(), 0);

    let mut transaction = CreateReceiveTokenTransaction {
        id: Uuid::new_v4(),
        transaction_hash: Some(token_transaction_ctx.transaction_hash.to_hex_string()),
        message_hash: token_transaction_ctx.message_hash.to_hex_string(),
        account_workchain_id: receiver_address.get_workchain_id(),
        account_hex: receiver_address.address().to_hex_string(),
        sender_workchain_id: None,
        sender_hex: None,
        value: amount,
        root_address: receiver_info.root_address.to_string(),
        payload: None,
        error: None,
        block_hash: token_transaction_ctx.block_hash.to_hex_string(),
        block_time: token_transaction_ctx.block_utime as i32,
        direction: TonTransactionDirection::Receive,
        status: TonTokenTransactionStatus::New,
    };

    if TOKEN_WALLET_CODE_HASH.as_ref() != receiver_info.code_hash {
        transaction.error = Some("Bad hash".to_string())
    }

    Ok(ReceiveTokenTransaction::Create(transaction))
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
        .ok_or_else(|| TonCoreError::AccountNotFound(account.to_hex_string()))?;

    let state = nekoton::core::token_wallet::TokenWalletContractState(&state);
    let hash = *state.get_code_hash()?.as_slice();
    let version = state.get_version()?;
    let details = state.get_details(version)?;
    Ok((details, hash))
}
