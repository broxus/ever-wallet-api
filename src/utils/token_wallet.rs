use anyhow::Result;
use bigdecimal::BigDecimal;
use nekoton::core::models::{
    RootTokenContractDetails, TokenWalletDetails, TokenWalletVersion, TransferRecipient,
};
use nekoton::core::token_wallet::{RootTokenContractState, TokenWalletContractState};
use nekoton::core::InternalMessage;
use nekoton::transport::models::ExistingContract;
use nekoton_abi::{BigUint128, BigUint256, MessageBuilder};
use nekoton_contracts::{old_tip3, tip3_1};
use nekoton_utils::SimpleClock;
use num_bigint::BigUint;
use ton_block::MsgAddressInt;
use ton_types::UInt256;

const INITIAL_BALANCE: u64 = 100_000_000; // 0.1 TON

pub fn prepare_token_transfer(
    owner: MsgAddressInt,
    token_wallet: MsgAddressInt,
    version: TokenWalletVersion,
    destination: TransferRecipient,
    tokens: BigUint,
    send_gas_to: MsgAddressInt,
    notify_receiver: bool,
    attached_amount: u64,
    payload: ton_types::Cell,
) -> Result<InternalMessage> {
    let (function, input) = match version {
        TokenWalletVersion::OldTip3v4 => {
            use old_tip3::token_wallet_contract;
            match destination {
                TransferRecipient::TokenWallet(token_wallet) => {
                    MessageBuilder::new(token_wallet_contract::transfer())
                        .arg(token_wallet) // to
                        .arg(BigUint128(tokens)) // tokens
                }
                TransferRecipient::OwnerWallet(owner_wallet) => {
                    MessageBuilder::new(token_wallet_contract::transfer_to_recipient())
                        .arg(BigUint256(Default::default())) // recipient_public_key
                        .arg(owner_wallet) // recipient_address
                        .arg(BigUint128(tokens)) // tokens
                        .arg(BigUint128(INITIAL_BALANCE.into())) // deploy_grams
                }
            }
            .arg(BigUint128(Default::default())) // grams / transfer_grams
            .arg(send_gas_to) // send_gas_to
            .arg(notify_receiver) // notify_receiver
            .arg(payload) // payload
            .build()
        }
        TokenWalletVersion::Tip3 => {
            use tip3_1::token_wallet_contract;
            match destination {
                TransferRecipient::TokenWallet(token_wallet) => {
                    MessageBuilder::new(token_wallet_contract::transfer_to_wallet())
                        .arg(BigUint128(tokens)) // amount
                        .arg(token_wallet) // recipient token wallet
                }
                TransferRecipient::OwnerWallet(owner_wallet) => {
                    MessageBuilder::new(token_wallet_contract::transfer())
                        .arg(BigUint128(tokens)) // amount
                        .arg(owner_wallet) // recipient
                        .arg(BigUint128(INITIAL_BALANCE.into())) // deployWalletValue
                }
            }
            .arg(send_gas_to) // remainingGasTo
            .arg(notify_receiver) // notify
            .arg(payload) // payload
            .build()
        }
    };

    let body = function.encode_internal_input(&input)?.into();

    Ok(InternalMessage {
        source: Some(owner),
        destination: token_wallet,
        amount: attached_amount,
        bounce: true,
        body,
    })
}

pub fn prepare_token_burn(
    owner: MsgAddressInt,
    token_wallet: MsgAddressInt,
    version: TokenWalletVersion,
    tokens: BigUint,
    send_gas_to: MsgAddressInt,
    callback_to: MsgAddressInt,
    attached_amount: u64,
    payload: ton_types::Cell,
) -> Result<InternalMessage> {
    let (function, input) = match version {
        TokenWalletVersion::OldTip3v4 => {
            use old_tip3::token_wallet_contract;
            MessageBuilder::new(token_wallet_contract::burn_by_owner())
                .arg(BigUint128(tokens)) // amount
                .arg(0) // grams
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

    let body = function.encode_internal_input(&input)?.into();

    Ok(InternalMessage {
        source: Some(owner),
        destination: token_wallet,
        amount: attached_amount,
        bounce: true,
        body,
    })
}

pub fn prepare_token_mint(
    owner: MsgAddressInt,
    root_token: MsgAddressInt,
    version: TokenWalletVersion,
    tokens: BigUint,
    recipient: MsgAddressInt,
    deploy_wallet_value: BigUint,
    send_gas_to: MsgAddressInt,
    notify: bool,
    attached_amount: u64,
    payload: ton_types::Cell,
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

    let body = function.encode_internal_input(&input)?.into();

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
    owner: &MsgAddressInt,
) -> Result<MsgAddressInt> {
    let root_contract_state = RootTokenContractState(root_contract);
    let RootTokenContractDetails { version, .. } =
        root_contract_state.guess_details(&SimpleClock)?;

    root_contract_state.get_wallet_address(&SimpleClock, version, owner)
}

pub fn get_token_wallet_account(
    root_contract: &ExistingContract,
    owner: &MsgAddressInt,
) -> Result<UInt256> {
    let root_contract_state = RootTokenContractState(root_contract);
    let RootTokenContractDetails { version, .. } =
        root_contract_state.guess_details(&SimpleClock)?;

    let token_wallet_address =
        root_contract_state.get_wallet_address(&SimpleClock, version, owner)?;
    let token_wallet_account =
        UInt256::from_be_bytes(&token_wallet_address.address().get_bytestring(0));

    Ok(token_wallet_account)
}

pub fn get_token_wallet_basic_info(
    token_contract: &ExistingContract,
) -> Result<(TokenWalletVersion, BigDecimal)> {
    let token_wallet_state = TokenWalletContractState(token_contract);

    let version = token_wallet_state.get_version(&SimpleClock)?;
    let balance = BigDecimal::new(
        token_wallet_state
            .get_balance(&SimpleClock, version)?
            .into(),
        0,
    );

    Ok((version, balance))
}

pub fn get_token_wallet_details(
    token_contract: &ExistingContract,
) -> Result<(TokenWalletDetails, TokenWalletVersion, [u8; 32])> {
    let contract_state = nekoton::core::token_wallet::TokenWalletContractState(token_contract);

    let hash = *contract_state.get_code_hash()?.as_slice();
    let version = contract_state.get_version(&SimpleClock)?;
    let details = contract_state.get_details(&SimpleClock, version)?;

    Ok((details, version, hash))
}

pub fn get_root_token_version(root_contract: &ExistingContract) -> Result<TokenWalletVersion> {
    let root_contract_state = RootTokenContractState(root_contract);
    let RootTokenContractDetails { version, .. } =
        root_contract_state.guess_details(&SimpleClock)?;

    Ok(version)
}

#[derive(thiserror::Error, Debug)]
enum TokenWalletError {
    #[error("Mint not supported by OldTip3v4 tokens")]
    MintNotSupported,
}
