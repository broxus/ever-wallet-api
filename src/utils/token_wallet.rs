use anyhow::Result;
use bigdecimal::BigDecimal;
use nekoton::core::models::*;
use nekoton::core::token_wallet::*;
use nekoton::core::*;
use nekoton::transport::models::*;
use nekoton_abi::*;
use nekoton_utils::*;
use num_bigint::BigUint;
use ton_block::MsgAddressInt;
use ton_types::UInt256;

use crate::utils::*;

const INITIAL_BALANCE: u64 = 100_000_000; // 0.1 TON

pub fn prepare_token_transfer(
    owner: MsgAddressInt,
    token_wallet: MsgAddressInt,
    destination: TransferRecipient,
    send_gas_to: MsgAddressInt,
    version: TokenWalletVersion,
    tokens: BigUint,
    notify_receiver: bool,
    attached_amount: u64,
    payload: ton_types::Cell,
) -> Result<InternalMessage> {
    let contract = select_token_contract(version)?;

    let (function, input) = match destination {
        TransferRecipient::TokenWallet(token_wallet) => {
            MessageBuilder::new(contract, "transfer")
                .trust_me()
                .arg(token_wallet) // to
                .arg(BigUint128(tokens)) // tokens
        }
        TransferRecipient::OwnerWallet(owner_wallet) => {
            MessageBuilder::new(contract, "transferToRecipient")
                .trust_me()
                .arg(BigUint256(Default::default())) // recipient_public_key
                .arg(owner_wallet) // recipient_address
                .arg(BigUint128(tokens)) // tokens
                .arg(BigUint128(INITIAL_BALANCE.into())) // deploy_grams
        }
    }
    .arg(BigUint128(Default::default())) // grams / transfer_grams
    .arg(&send_gas_to) // send_gas_to
    .arg(notify_receiver) // notify_receiver
    .arg(payload) // payload
    .build();

    let body = function
        .encode_input(&Default::default(), &input, true, None)?
        .into();

    Ok(InternalMessage {
        source: Some(owner),
        destination: token_wallet,
        amount: attached_amount,
        bounce: true,
        body,
    })
}

pub fn get_token_wallet_address(
    root_contract: ExistingContract,
    owner: &MsgAddressInt,
) -> Result<MsgAddressInt> {
    let root_contract_state = RootTokenContractState(&root_contract);
    let RootTokenContractDetails { version, .. } = root_contract_state.guess_details()?;

    root_contract_state.get_wallet_address(version, owner, None)
}

pub fn get_token_wallet_account(
    root_contract: &ExistingContract,
    owner: &MsgAddressInt,
) -> Result<UInt256> {
    let root_contract_state = RootTokenContractState(root_contract);
    let RootTokenContractDetails { version, .. } = root_contract_state.guess_details()?;

    let token_wallet_address = root_contract_state.get_wallet_address(version, owner, None)?;
    let token_wallet_account =
        UInt256::from_be_bytes(&token_wallet_address.address().get_bytestring(0));

    Ok(token_wallet_account)
}

pub fn get_token_wallet_basic_info(
    token_contract: &ExistingContract,
) -> Result<(TokenWalletVersion, BigDecimal)> {
    let token_wallet_state = TokenWalletContractState(token_contract);

    let version = token_wallet_state.get_version()?;
    let balance = BigDecimal::new(token_wallet_state.get_balance(version)?.into(), 0);

    Ok((version, balance))
}

pub fn get_token_wallet_details(
    address: &MsgAddressInt,
    shard_accounts: &ton_block::ShardAccounts,
) -> Result<(TokenWalletDetails, [u8; 32])> {
    let account = UInt256::from_be_bytes(&address.address().get_bytestring(0));
    let state = shard_accounts
        .find_account(&account)?
        .ok_or_else(|| TokenWalletError::AccountNotExist(account.to_hex_string()))?;

    let state = nekoton::core::token_wallet::TokenWalletContractState(&state);
    let hash = *state.get_code_hash()?.as_slice();
    let version = state.get_version()?;
    let details = state.get_details(version)?;
    Ok((details, hash))
}

fn select_token_contract(version: TokenWalletVersion) -> Result<&'static ton_abi::Contract> {
    Ok(match version {
        TokenWalletVersion::Tip3v1 => return Err(TokenWalletError::UnsupportedVersion.into()),
        TokenWalletVersion::Tip3v2 => nekoton_contracts::abi::ton_token_wallet_v2(),
        TokenWalletVersion::Tip3v3 => nekoton_contracts::abi::ton_token_wallet_v3(),
        TokenWalletVersion::Tip3v4 => nekoton_contracts::abi::ton_token_wallet_v4(),
    })
}

#[derive(thiserror::Error, Debug)]
enum TokenWalletError {
    #[error("Unsupported version")]
    UnsupportedVersion,
    #[error("Account `{0}` not exist")]
    AccountNotExist(String),
}
