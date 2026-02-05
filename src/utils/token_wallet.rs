use anyhow::Result;
use bigdecimal::BigDecimal;
use nekoton_abi::ExecutionContext;
use nekoton_contracts::tip3_any::{RootTokenContractState, TokenWalletContractState};
use nekoton_utils::SimpleClock;
use num_bigint::BigUint;
use num_traits::ToPrimitive;
use tycho_types::abi::AbiValue;
use tycho_types::cell::{Cell, HashBytes};
use tycho_types::models::StdAddr;

use crate::models::ExistingContract;
use crate::utils::token_wallets::models::{
    RootTokenContractDetails, TokenWalletDetails, TokenWalletVersion,
};

const INITIAL_BALANCE: u128 = 100_000_000; // 0.1

#[derive(Clone, Debug)]
pub struct InternalMessage {
    pub source: Option<StdAddr>,
    pub destination: StdAddr,
    pub amount: u128,
    pub bounce: bool,
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
    let (function, tokens) = match version {
        TokenWalletVersion::OldTip3v4 => {
            return Err(TokenWalletError::NotSupported.into());
        }
        TokenWalletVersion::Tip3 => {
            use crate::utils::token_wallets;
            let function = token_wallets::transfer();
            let tokens = [
                AbiValue::uint(128, tokens.to_u128().unwrap()).named("amount"),
                AbiValue::address(destination).named("recipient"),
                AbiValue::uint(128, INITIAL_BALANCE).named("deployWalletValue"),
                AbiValue::address(send_gas_to).named("remainingGasTo"),
                AbiValue::Bool(notify_receiver).named("notify"),
                AbiValue::Cell(payload).named("payload"),
            ]
            .to_vec();
            (function, tokens)
        }
    };

    let external_input = function.encode_external(&tokens);
    let (_, body) = external_input.build_input_without_signature()?;

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
    let (function, tokens) = match version {
        TokenWalletVersion::OldTip3v4 => {
            return Err(TokenWalletError::NotSupported.into());
        }
        TokenWalletVersion::Tip3 => {
            use crate::utils::token_wallets;

            let function = token_wallets::burnable::burn();
            let tokens = [
                AbiValue::uint(128, tokens.to_u128().unwrap()).named("amount"),
                AbiValue::address(send_gas_to).named("remainingGasTo"),
                AbiValue::address(callback_to).named("callbackTo"),
                AbiValue::Cell(payload).named("payload"),
            ]
            .to_vec();
            (function, tokens)
        }
    };

    let external_input = function.encode_external(&tokens);
    let (_, body) = external_input.build_input_without_signature()?;

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
    let (function, tokens) = match version {
        TokenWalletVersion::OldTip3v4 => return Err(TokenWalletError::NotSupported.into()),
        TokenWalletVersion::Tip3 => {
            use crate::utils::token_wallets;
            let function = token_wallets::mint();
            let tokens = [
                AbiValue::uint(128, tokens.to_u128().unwrap()).named("amount"),
                AbiValue::address(recipient).named("recipient"),
                AbiValue::uint(128, deploy_wallet_value).named("deployWalletValue"),
                AbiValue::address(send_gas_to).named("remainingGasTo"),
                AbiValue::Bool(notify).named("notify"),
                AbiValue::Cell(payload).named("payload"),
            ]
            .to_vec();
            (function, tokens)
        }
    };

    let external_input = function.encode_external(&tokens);
    let (_, body) = external_input.build_input_without_signature()?;

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
    #[error("not supported OldTip3v4 tokens")]
    NotSupported,
}
