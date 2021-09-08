use anyhow::Result;
use nekoton::core::models::*;
use nekoton::core::*;
use nekoton_abi::*;
use nekoton_utils::*;
use num_bigint::BigUint;
use ton_block::MsgAddressInt;

const INITIAL_BALANCE: u64 = 100_000_000; // 0.1 TON

pub fn prepare_token_transfer(
    owner: MsgAddressInt,
    token_wallet: MsgAddressInt,
    destination: TransferRecipient,
    version: TokenWalletVersion,
    tokens: BigUint,
    notify_receiver: bool,
    payload: ton_types::Cell,
) -> Result<InternalMessage> {
    const ATTACHED_AMOUNT: u64 = 500_000_000; // 0.5 TON

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
    .arg(&owner) // send_gas_to
    .arg(notify_receiver) // notify_receiver
    .arg(payload) // payload
    .build();

    let body = function
        .encode_input(&Default::default(), &input, true, None)?
        .into();

    Ok(InternalMessage {
        source: Some(owner),
        destination: token_wallet,
        amount: ATTACHED_AMOUNT,
        bounce: true,
        body,
    })
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
}
