
use anyhow::Result;
use ed25519_dalek::PublicKey;
use tycho_types::{abi::{AbiHeaderType, AbiVersion, Function, NamedAbiValue, UnsignedExternalMessage}, cell::{CellBuilder, HashBytes}, models::{Account, AccountState, IntAddr, IntMsgInfo, Message, MessageLayout, MsgInfo, StateInit, StdAddr}};

use crate::utils::ton_wallet::{Gift, TonWalletDetails, ever_wallet};


pub fn prepare_deploy(
    public_key: &PublicKey,
    workchain: i8,
    expire_at: u32,
) -> Result<UnsignedExternalMessage> {
    let state_init = prepare_state_init(public_key)?;
    let hash = CellBuilder::build_from(&state_init)?.repr_hash();

    let dst = StdAddr::new(
        workchain,
        hash.into(),
    );
    
    let headers = vec![AbiHeaderType::Time, AbiHeaderType::Expire, AbiHeaderType::Pubkey];
    let function = Function::builder(AbiVersion::V2_3, "sendTransactionRaw")
        .with_headers(headers)
        .with_inputs(vec![])
        .with_outputs(vec![])
        .with_id(0x169e3e11)
        .build();

    let unsigned_body = function.encode_external(&[]).with_expire_at(expire_at).build_input()?;
    let mut unsigned_message = unsigned_body.with_dst(dst);
    unsigned_message.set_state_init(Some(state_init));
    Ok(unsigned_message)
}

pub fn prepare_transfer(
    public_key: &PublicKey,
    current_state: &Account,
    address: StdAddr,
    gifts: Vec<Gift>,
    expire_at: u32,
) -> Result<UnsignedExternalMessage> {
    use crate::utils::wallets::ever_wallet;

    if gifts.len() > MAX_MESSAGES {
        return Err(EverWalletError::TooManyGifts.into());
    }

    let mut gifts = gifts.into_iter();
    let body = match (gifts.len(), gifts.next()) {
        (1, Some(gift)) if gift.state_init.is_none() => {
            let function = ever_wallet::send_transaction();
            function.encode_external(&[
                NamedAbiValue::from(("destination", gift.destination)),
                NamedAbiValue::from(("amount", gift.amount.into())),
                NamedAbiValue::from(("bounce", gift.bounce)),
                NamedAbiValue::from(("flags", gift.flags)),
                NamedAbiValue::from(("body", gift.body.unwrap_or_default().into_cell())),
            ])?
        }
        (len, gift) => {
            let function = match len {
                0 => ever_wallet::send_transaction_raw_0(),
                1 => ever_wallet::send_transaction_raw_1(),
                2 => ever_wallet::send_transaction_raw_2(),
                3 => ever_wallet::send_transaction_raw_3(),
                _ => ever_wallet::send_transaction_raw_4(),
            };

            let mut tokens = Vec::with_capacity(len * 2);
            for gift in gift.into_iter().chain(gifts) {
                let internal_message = Message {
                    info: MsgInfo::Int(IntMsgInfo {
                        ihr_disabled: true,
                        bounce: gift.bounce,
                        dst: gift.destination,
                        value: gift.amount.into(),
                        ..Default::default()
                    }),
                    init: gift.state_init,
                    body: gift.body.unwrap_or(Default::default()).as_slice()?,
                    layout: None,
                };

                tokens.push(NamedAbiValue::from(("flags", gift.flags.token_value())));
                tokens.push(NamedAbiValue::from((
                    "message",
                    CellBuilder::build_from(internal_message.borrow())?
                )));
            }
            function.encode_external(&tokens)?
        }
    };


    let unsigned_body = function.encode_external(&[]).with_expire_at(expire_at).build_input()?;
    let mut unsigned_message = unsigned_body.with_dst(address);

    match &current_state.state {
        AccountState::Active { .. } => {}
        AccountState::Frozen { .. } => {
            return Err(EverWalletError::AccountIsFrozen.into())
        }
        AccountState::Uninit => {
            unsigned_message.set_state_init(Some(prepare_state_init(public_key)?));
        }
    };

    Ok(unsigned_message)
}

pub static CODE_HASH: &[u8; 32] = &[
    0x3b, 0xa6, 0x52, 0x8a, 0xb2, 0x69, 0x4c, 0x11, 0x81, 0x80, 0xaa, 0x3b, 0xd1, 0x0d, 0xd1, 0x9f,
    0xf4, 0x00, 0xb9, 0x09, 0xab, 0x4d, 0xcf, 0x58, 0xfc, 0x69, 0x92, 0x5b, 0x2c, 0x7b, 0x12, 0xa6,
];

pub fn is_ever_wallet(code_hash: &HashBytes) -> bool {
    code_hash.as_slice() == CODE_HASH
}

pub fn compute_contract_address(public_key: &PublicKey, workchain_id: i8) -> IntAddr {
    let hash = prepare_state_init(public_key)
        .and_then(|state| state.hash())
        .trust_me();
    IntAddr::Std(StdAddr::new(
        workchain_id,
        hash.into(),
    ))
}

pub fn prepare_state_init(public_key: &PublicKey) -> Result<StateInit> {
    let mut builder = CellBuilder::new();
    builder.store_u256(public_key.as_bytes())?;
    builder.store_u64(0)?;

    let data = builder.build()?;

    Ok(StateInit {
        code: Some(ever_wallet()),
        data: Some(data),
        ..Default::default()
    })
}

pub static DETAILS: TonWalletDetails = TonWalletDetails {
    requires_separate_deploy: false,
    min_amount: 1, // 0.000000001 EVER
    max_messages: MAX_MESSAGES,
    supports_payload: true,
    supports_state_init: true,
    supports_multiple_owners: false,
    supports_code_update: false,
    expiration_time: 0,
    required_confirmations: None,
};

const MAX_MESSAGES: usize = 4;

#[derive(thiserror::Error, Debug)]
enum EverWalletError {
    #[error("Account is frozen")]
    AccountIsFrozen,
    #[error("Too many outgoing messages")]
    TooManyGifts,
}
