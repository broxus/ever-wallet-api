use std::convert::TryFrom;

use anyhow::Result;
use ed25519_dalek::PublicKey;
use ton_block::{MsgAddrStd, MsgAddressInt, Serializable};
use ton_types::{BuilderData, Cell, IBitstring, SliceData, UInt256};

use nekoton_utils::*;

use super::{Gift, TonWalletDetails, TransferAction};
use crate::core::models::{Expiration, ExpireAt, PendingTransaction};
use crate::crypto::{SignedMessage, UnsignedMessage};

pub fn prepare_deploy(
    clock: &dyn Clock,
    public_key: &PublicKey,
    workchain: i8,
    expiration: Expiration,
) -> Result<Box<dyn UnsignedMessage>> {
    let init_data = InitData::from_key(public_key).with_wallet_id(WALLET_ID);
    let dst = compute_contract_address(public_key, workchain);
    let mut message =
        ton_block::Message::with_ext_in_header(ton_block::ExternalInboundMessageHeader {
            dst,
            ..Default::default()
        });

    message.set_state_init(init_data.make_state_init()?);

    let expire_at = ExpireAt::new(clock, expiration);
    let (hash, payload) = init_data.make_transfer_payload(None, expire_at.timestamp)?;

    Ok(Box::new(UnsignedWalletV3Message {
        init_data,
        gifts: Vec::new(),
        payload,
        message,
        expire_at,
        hash,
    }))
}

pub fn prepare_state_init(public_key: &PublicKey) -> Result<ton_block::StateInit> {
    let init_data = InitData::from_key(public_key).with_wallet_id(WALLET_ID);
    init_data.make_state_init()
}

/// Adjusts seqno if there are some recent pending transactions that have not expired
pub fn estimate_seqno_offset(
    clock: &dyn Clock,
    current_state: &ton_block::AccountStuff,
    pending_transactions: &[PendingTransaction],
) -> u32 {
    const SEQNO_ADJUST_INTERVAL: u32 = 30; // seconds

    #[inline]
    fn same_lt(lt_from_pending: u64, lt_from_state: u64) -> bool {
        // NOTE: `pending.latest_lt` can be exact transaction lt, or
        // `storage.last_trans_lt` which is a bit greater
        const ALLOWED_LT_DIFF: u64 = 1 + MAX_MESSAGES as u64;

        (lt_from_pending..=lt_from_pending + ALLOWED_LT_DIFF).contains(&lt_from_state)
    }

    if pending_transactions.is_empty() {
        return 0;
    }

    let now = clock.now_sec_u64() as u32;
    let latest_lt = current_state.storage.last_trans_lt;

    let mut seqno_offset = 0;
    for pending in pending_transactions.iter().rev() {
        // Adjust only for sufficiently new pending transactions.
        if now > pending.created_at + SEQNO_ADJUST_INTERVAL {
            break;
        }

        // Adjust only if account state hasn't changed
        if !same_lt(pending.latest_lt, latest_lt) {
            break;
        }

        if now < pending.expire_at {
            seqno_offset += 1;
        }
    }

    seqno_offset
}

pub fn prepare_transfer(
    clock: &dyn Clock,
    public_key: &PublicKey,
    current_state: &ton_block::AccountStuff,
    seqno_offset: u32,
    gifts: Vec<Gift>,
    expiration: Expiration,
) -> Result<TransferAction> {
    if gifts.len() > MAX_MESSAGES {
        return Err(WalletV3Error::TooManyGifts.into());
    }

    let (mut init_data, with_state_init) = match &current_state.storage.state {
        ton_block::AccountState::AccountActive { state_init, .. } => match &state_init.data {
            Some(data) => (InitData::try_from(data)?, false),
            None => return Err(WalletV3Error::InvalidInitData.into()),
        },
        ton_block::AccountState::AccountFrozen { .. } => {
            return Err(WalletV3Error::AccountIsFrozen.into())
        }
        ton_block::AccountState::AccountUninit => (
            InitData::from_key(public_key).with_wallet_id(WALLET_ID),
            true,
        ),
    };

    init_data.seqno += seqno_offset;

    let mut message =
        ton_block::Message::with_ext_in_header(ton_block::ExternalInboundMessageHeader {
            dst: current_state.addr.clone(),
            ..Default::default()
        });

    if with_state_init {
        message.set_state_init(init_data.make_state_init()?);
    }

    let expire_at = ExpireAt::new(clock, expiration);
    let (hash, payload) = init_data.make_transfer_payload(gifts.clone(), expire_at.timestamp)?;

    Ok(TransferAction::Sign(Box::new(UnsignedWalletV3Message {
        init_data,
        gifts,
        payload,
        hash,
        expire_at,
        message,
    })))
}

#[derive(Clone)]
struct UnsignedWalletV3Message {
    init_data: InitData,
    gifts: Vec<Gift>,
    payload: BuilderData,
    hash: UInt256,
    expire_at: ExpireAt,
    message: ton_block::Message,
}

impl UnsignedMessage for UnsignedWalletV3Message {
    fn refresh_timeout(&mut self, clock: &dyn Clock) {
        if !self.expire_at.refresh(clock) {
            return;
        }

        let (hash, payload) = self
            .init_data
            .make_transfer_payload(self.gifts.clone(), self.expire_at())
            .trust_me();
        self.hash = hash;
        self.payload = payload;
    }

    fn expire_at(&self) -> u32 {
        self.expire_at.timestamp
    }

    fn hash(&self) -> &[u8] {
        self.hash.as_slice()
    }

    fn sign(&self, signature: &[u8; ed25519_dalek::SIGNATURE_LENGTH]) -> Result<SignedMessage> {
        let mut payload = self.payload.clone();
        payload.prepend_raw(signature, signature.len() * 8)?;

        let mut message = self.message.clone();
        message.set_body(SliceData::load_builder(payload)?);

        Ok(SignedMessage {
            message,
            expire_at: self.expire_at(),
        })
    }

    fn sign_with_pruned_payload(
        &self,
        signature: &[u8; ed25519_dalek::SIGNATURE_LENGTH],
        prune_after_depth: u16,
    ) -> Result<SignedMessage> {
        let mut payload = self.payload.clone();
        payload.prepend_raw(signature, signature.len() * 8)?;
        let body = payload.into_cell()?;

        let mut message = self.message.clone();
        message.set_body(prune_deep_cells(&body, prune_after_depth)?);

        Ok(SignedMessage {
            message,
            expire_at: self.expire_at(),
        })
    }
}

pub static CODE_HASH: &[u8; 32] = &[
    0x84, 0xda, 0xfa, 0x44, 0x9f, 0x98, 0xa6, 0x98, 0x77, 0x89, 0xba, 0x23, 0x23, 0x58, 0x07, 0x2b,
    0xc0, 0xf7, 0x6d, 0xc4, 0x52, 0x40, 0x02, 0xa5, 0xd0, 0x91, 0x8b, 0x9a, 0x75, 0xd2, 0xd5, 0x99,
];

pub fn is_wallet_v3(code_hash: &UInt256) -> bool {
    code_hash.as_slice() == CODE_HASH
}

pub fn compute_contract_address(public_key: &PublicKey, workchain_id: i8) -> MsgAddressInt {
    InitData::from_key(public_key)
        .with_wallet_id(WALLET_ID)
        .compute_addr(workchain_id)
        .trust_me()
}

pub static DETAILS: TonWalletDetails = TonWalletDetails {
    requires_separate_deploy: false,
    min_amount: 1, // 0.000000001 TON
    max_messages: MAX_MESSAGES,
    supports_payload: true,
    supports_state_init: true,
    supports_multiple_owners: false,
    supports_code_update: false,
    expiration_time: 0,
    required_confirmations: None,
};

const MAX_MESSAGES: usize = 4;

/// `WalletV3` init data
#[derive(Clone, Copy)]
pub struct InitData {
    pub seqno: u32,
    pub wallet_id: u32,
    pub public_key: UInt256,
}

impl InitData {
    pub fn public_key(&self) -> &[u8; 32] {
        self.public_key.as_slice()
    }

    pub fn from_key(key: &PublicKey) -> Self {
        Self {
            seqno: 0,
            wallet_id: 0,
            public_key: key.as_bytes().into(),
        }
    }

    pub fn with_wallet_id(mut self, id: u32) -> Self {
        self.wallet_id = id;
        self
    }

    pub fn compute_addr(&self, workchain_id: i8) -> Result<MsgAddressInt> {
        let init_state = self.make_state_init()?.serialize()?;
        let hash = init_state.repr_hash();
        Ok(MsgAddressInt::AddrStd(MsgAddrStd {
            anycast: None,
            workchain_id,
            address: hash.into(),
        }))
    }

    pub fn make_state_init(&self) -> Result<ton_block::StateInit> {
        Ok(ton_block::StateInit {
            code: Some(nekoton_contracts::wallets::code::wallet_v3()),
            data: Some(self.serialize()?),
            ..Default::default()
        })
    }

    pub fn serialize(&self) -> Result<Cell> {
        let mut data = BuilderData::new();
        data.append_u32(self.seqno)?
            .append_u32(self.wallet_id)?
            .append_raw(self.public_key.as_slice(), 256)?;
        data.into_cell()
    }

    pub fn make_transfer_payload(
        &self,
        gifts: impl IntoIterator<Item = Gift>,
        expire_at: u32,
    ) -> Result<(UInt256, BuilderData)> {
        let mut payload = BuilderData::new();

        // insert prefix
        payload
            .append_u32(self.wallet_id)?
            .append_u32(expire_at)?
            .append_u32(self.seqno)?;

        // create internal message
        for gift in gifts {
            let mut internal_message =
                ton_block::Message::with_int_header(ton_block::InternalMessageHeader {
                    ihr_disabled: true,
                    bounce: gift.bounce,
                    dst: gift.destination,
                    value: ton_block::CurrencyCollection::from_grams(ton_block::Grams::new(
                        gift.amount,
                    )?),
                    ..Default::default()
                });

            if let Some(body) = gift.body {
                internal_message.set_body(body);
            }

            if let Some(state_init) = gift.state_init {
                internal_message.set_state_init(state_init);
            }

            // append it to the body
            payload
                .append_u8(gift.flags)?
                .checked_append_reference(internal_message.serialize()?)?;
        }

        let hash = payload.clone().into_cell()?.repr_hash();

        Ok((hash, payload))
    }
}

impl TryFrom<&Cell> for InitData {
    type Error = anyhow::Error;

    fn try_from(data: &Cell) -> Result<Self, Self::Error> {
        let mut cs = SliceData::load_cell_ref(data)?;
        Ok(Self {
            seqno: cs.get_next_u32()?,
            wallet_id: cs.get_next_u32()?,
            public_key: UInt256::from_be_bytes(&cs.get_next_bytes(32)?),
        })
    }
}

const WALLET_ID: u32 = 0x4BA92D8A;

#[derive(thiserror::Error, Debug)]
enum WalletV3Error {
    #[error("Invalid init data")]
    InvalidInitData,
    #[error("Account is frozen")]
    AccountIsFrozen,
    #[error("Too many outgoing messages")]
    TooManyGifts,
}
