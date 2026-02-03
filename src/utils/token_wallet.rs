use anyhow::Result;
use bigdecimal::BigDecimal;
use nekoton::core::models::{RootTokenContractDetails, TokenWalletDetails, TokenWalletVersion};
use nekoton_abi::{BigUint128, BigUint256, ExecutionContext, MessageBuilder};
use nekoton_contracts::tip3_any::{RootTokenContractState, TokenWalletContractState};
use nekoton_contracts::{old_tip3, tip3_1};
use nekoton_utils::SimpleClock;
use num_bigint::BigUint;
use tycho_types::cell::{Cell, HashBytes};
use tycho_types::models::StdAddr;

use crate::models::ExistingContract;

const INITIAL_BALANCE: u64 = 100_000_000; // 0.1 TON

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InternalMessage {
    #[serde(
        with = "serde_optional_address",
        skip_serializing_if = "Option::is_none"
    )]
    pub source: Option<StdAddr>,
    #[serde(with = "serde_address")]
    pub destination: StdAddr,
    #[serde(with = "serde_string")]
    pub amount: u128,
    pub bounce: bool,
    #[serde(with = "serde_boc")]
    pub body: Cell,
}

pub fn prepare_token_transfer(
    owner: StdAddr,
    token_wallet: StdAddr,
    version: TokenWalletVersion,
    destination: StdAddr,
    tokens: BigUint,
    send_gas_to: StdAddr,
    notify_receiver: bool,
    attached_amount: u128,
    payload: Cell,
) -> Result<InternalMessage> {
    let (function, input) = match version {
        TokenWalletVersion::OldTip3v4 => {
            use old_tip3::token_wallet_contract;
            MessageBuilder::new(token_wallet_contract::transfer_to_recipient())
                .arg(BigUint256(Default::default())) // recipient_public_key
                .arg(owner_wallet) // recipient_address
                .arg(BigUint128(tokens)) // tokens
                .arg(BigUint128(INITIAL_BALANCE.into())) // deploy_grams
                .arg(BigUint128(Default::default())) // grams / transfer_grams
                .arg(send_gas_to) // send_gas_to
                .arg(notify_receiver) // notify_receiver
                .arg(payload) // payload
                .build()
        }
        TokenWalletVersion::Tip3 => {
            use tip3_1::token_wallet_contract;
            MessageBuilder::new(token_wallet_contract::transfer())
                .arg(BigUint128(tokens)) // amount
                .arg(owner_wallet) // recipient
                .arg(BigUint128(INITIAL_BALANCE.into())) // deployWalletValue
                .arg(send_gas_to) // remainingGasTo
                .arg(notify_receiver) // notify
                .arg(payload) // payload
                .build()
        }
    };

    let body = SliceData::load_builder(function.encode_internal_input(&input)?)?;

    Ok(InternalMessage {
        source: Some(owner),
        destination: token_wallet,
        amount: attached_amount,
        bounce: true,
        body,
    })
}

pub fn prepare_token_burn(
    owner: StdAddr,
    token_wallet: StdAddr,
    version: TokenWalletVersion,
    tokens: BigUint,
    send_gas_to: StdAddr,
    callback_to: StdAddr,
    attached_amount: u128,
    payload: Cell,
) -> Result<InternalMessage> {
    let (function, input) = match version {
        TokenWalletVersion::OldTip3v4 => {
            use old_tip3::token_wallet_contract;
            MessageBuilder::new(token_wallet_contract::burn_by_owner())
                .arg(BigUint128(tokens)) // amount
                .arg(0u128) // grams
                .arg(send_gas_to) // remainingGasTo
                .arg(callback_to) // callback_address
                .arg(payload) // payload
                .build()
        }
        TokenWalletVersion::Tip3 => {
            use tip3_1::token_wallet_contract;
            MessageBuilder::new(token_wallet_contract::burnable::burn())
                .arg(BigUint128(tokens)) // amount
                .arg(send_gas_to) // remainingGasTo
                .arg(callback_to) // callbackTo
                .arg(payload) // payload
                .build()
        }
    };

    let body = SliceData::load_builder(function.encode_internal_input(&input)?)?;

    Ok(InternalMessage {
        source: Some(owner),
        destination: token_wallet,
        amount: attached_amount,
        bounce: true,
        body,
    })
}

pub fn prepare_token_mint(
    owner: StdAddr,
    root_token: StdAddr,
    version: TokenWalletVersion,
    tokens: BigUint,
    recipient: StdAddr,
    deploy_wallet_value: BigUint,
    send_gas_to: StdAddr,
    notify: bool,
    attached_amount: u128,
    payload: Cell,
) -> Result<InternalMessage> {
    let (function, input) = match version {
        TokenWalletVersion::OldTip3v4 => return Err(TokenWalletError::MintNotSupported.into()),
        TokenWalletVersion::Tip3 => {
            use tip3_1::root_token_contract;
            MessageBuilder::new(root_token_contract::mint())
                .arg(BigUint128(tokens)) // amount
                .arg(recipient) // recipient
                .arg(BigUint128(deploy_wallet_value)) // deployWalletValue
                .arg(send_gas_to) // remainingGasTo
                .arg(notify) // notify
                .arg(payload) // payload
                .build()
        }
    };

    let body = SliceData::load_builder(function.encode_internal_input(&input)?)?;

    Ok(InternalMessage {
        source: Some(owner),
        destination: root_token,
        amount: attached_amount,
        bounce: true,
        body,
    })
}

pub fn get_token_wallet_address(
    root_contract: &ExistingContract,
    owner: &StdAddr,
) -> Result<StdAddr> {
    let root_contract_state = RootTokenContractState(ExecutionContext {
        clock: &SimpleClock,
        account_stuff: &root_contract.account,
        libraries: &[],
    });
    let RootTokenContractDetails { version, .. } = root_contract_state.guess_details()?;

    root_contract_state.get_wallet_address(version, owner)
}

pub fn get_token_wallet_account(
    root_contract: &ExistingContract,
    owner: &StdAddr,
) -> Result<HashBytes> {
    let root_contract_state = RootTokenContractState(ExecutionContext {
        clock: &SimpleClock,
        account_stuff: &root_contract.account,
        libraries: &[],
    });
    let RootTokenContractDetails { version, .. } = root_contract_state.guess_details()?;

    let token_wallet_address = root_contract_state.get_wallet_address(version, owner)?;
    let token_wallet_account = token_wallet_address.address;

    Ok(token_wallet_account)
}

pub fn get_token_wallet_basic_info(
    token_contract: &ExistingContract,
) -> Result<(TokenWalletVersion, BigDecimal)> {
    let token_wallet_state = TokenWalletContractState(ExecutionContext {
        clock: &SimpleClock,
        account_stuff: &token_contract.account,
        libraries: &[],
    });

    let version = token_wallet_state.get_version()?;
    let balance = BigDecimal::new(token_wallet_state.get_balance(version)?.into(), 0);

    Ok((version, balance))
}

pub fn get_token_wallet_details(
    token_contract: &ExistingContract,
) -> Result<(TokenWalletDetails, TokenWalletVersion, [u8; 32])> {
    let contract_state = TokenWalletContractState(ExecutionContext {
        clock: &SimpleClock,
        account_stuff: &token_contract.account,
        libraries: &[],
    });

    let hash = *contract_state.get_code_hash()?.as_slice();
    let version = contract_state.get_version()?;
    let details = contract_state.get_details(version)?;

    Ok((details, version, hash))
}

pub fn get_root_token_version(root_contract: &ExistingContract) -> Result<TokenWalletVersion> {
    let root_contract_state = RootTokenContractState(ExecutionContext {
        clock: &SimpleClock,
        account_stuff: &root_contract.account,
        libraries: &[],
    });
    let RootTokenContractDetails { version, .. } = root_contract_state.guess_details()?;

    Ok(version)
}

#[derive(thiserror::Error, Debug)]
enum TokenWalletError {
    #[error("Mint not supported by OldTip3v4 tokens")]
    MintNotSupported,
}
