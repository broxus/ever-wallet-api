use std::convert::TryFrom;

use anyhow::Result;
use ed25519_dalek::VerifyingKey;
use tycho_types::{
    cell::{Cell, CellBuilder, HashBytes},
    models::{
        Account, AccountState, CurrencyCollection, ExtInMsgInfo, IntAddr, MsgInfo, OwnedMessage,
        OwnedRelaxedMessage, RelaxedIntMsgInfo, RelaxedMsgInfo, StateInit, StdAddr,
    },
};
use tycho_util::time::now_sec;

use crate::utils::{
    ton_wallet::{Gift, PendingTransaction, TonWalletDetails},
    wallets::code::wallet_v3,
};

pub fn prepare_deploy(
    public_key: &VerifyingKey,
    workchain: i8,
    expire_at: u32,
) -> Result<UnsignedWalletV3Message> {
    let init_data = InitData::from_key(public_key).with_wallet_id(WALLET_ID);
    let dst = compute_contract_address(public_key, workchain)?;

    let (hash, payload) = init_data.make_transfer_payload(None, expire_at)?;
    let message = OwnedMessage {
        info: tycho_types::models::MsgInfo::ExtIn(ExtInMsgInfo {
            dst: IntAddr::Std(dst),
            ..Default::default()
        }),
        body: Default::default(),
        init: Some(init_data.make_state_init()?),
        layout: None,
    };
    Ok(UnsignedWalletV3Message {
        init_data,
        gifts: vec![],
        payload,
        hash,
        expire_at,
        message,
    })
}

pub fn prepare_state_init(public_key: &VerifyingKey) -> Result<StateInit> {
    let init_data = InitData::from_key(public_key).with_wallet_id(WALLET_ID);
    init_data.make_state_init()
}

/// Adjusts seqno if there are some recent pending transactions that have not expired
pub fn estimate_seqno_offset(
    current_state: &Account,
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

    let now = now_sec();
    let latest_lt = current_state.last_trans_lt;

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
    public_key: &VerifyingKey,
    current_state: &Account,
    seqno_offset: u32,
    gifts: Vec<Gift>,
    expire_at: u32,
) -> Result<UnsignedWalletV3Message> {
    if gifts.len() > MAX_MESSAGES {
        return Err(WalletV3Error::TooManyGifts.into());
    }

    let (mut init_data, with_state_init) = match &current_state.state {
        AccountState::Active(state_init) => match &state_init.data {
            Some(data) => (InitData::try_from(data)?, false),
            None => return Err(WalletV3Error::InvalidInitData.into()),
        },
        AccountState::Frozen { .. } => return Err(WalletV3Error::AccountIsFrozen.into()),
        AccountState::Uninit => (
            InitData::from_key(public_key).with_wallet_id(WALLET_ID),
            true,
        ),
    };

    init_data.seqno += seqno_offset;

    let (hash, payload) = init_data.make_transfer_payload(gifts.clone(), expire_at)?;
    let mut message = OwnedMessage {
        info: tycho_types::models::MsgInfo::ExtIn(ExtInMsgInfo {
            dst: IntAddr::Std(
                current_state
                    .address
                    .as_std()
                    .ok_or(WalletV3Error::InvalidAddress)?
                    .clone(),
            ),
            ..Default::default()
        }),
        body: Default::default(),
        init: None,
        layout: None,
    };

    if with_state_init {
        let state_init = init_data.make_state_init()?;
        message.init = Some(state_init);
    }

    Ok(UnsignedWalletV3Message {
        init_data,
        gifts,
        payload,
        hash,
        expire_at,
        message,
    })
}

pub struct UnsignedWalletV3Message {
    init_data: InitData,
    gifts: Vec<Gift>,
    payload: CellBuilder,
    hash: HashBytes,
    expire_at: u32,
    message: OwnedMessage,
}

impl UnsignedWalletV3Message {
    pub fn expire_at(&self) -> u32 {
        self.expire_at
    }

    pub fn hash(&self) -> &[u8] {
        self.hash.as_slice()
    }

    pub fn sign(&self, signature: &[u8; ed25519_dalek::SIGNATURE_LENGTH]) -> Result<OwnedMessage> {
        let mut payload = self.payload.clone();
        payload.prepend_raw(signature, (ed25519_dalek::SIGNATURE_LENGTH * 8) as u16)?;

        let mut message = self.message.clone();
        message.body = payload.build()?.into();

        Ok(message)
    }
}

pub static CODE_HASH: &[u8; 32] = &[
    0x84, 0xda, 0xfa, 0x44, 0x9f, 0x98, 0xa6, 0x98, 0x77, 0x89, 0xba, 0x23, 0x23, 0x58, 0x07, 0x2b,
    0xc0, 0xf7, 0x6d, 0xc4, 0x52, 0x40, 0x02, 0xa5, 0xd0, 0x91, 0x8b, 0x9a, 0x75, 0xd2, 0xd5, 0x99,
];

pub fn is_wallet_v3(code_hash: &HashBytes) -> bool {
    code_hash.as_slice() == CODE_HASH
}

pub fn compute_contract_address(public_key: &VerifyingKey, workchain_id: i8) -> Result<StdAddr> {
    let state_init = InitData::from_key(public_key)
        .with_wallet_id(WALLET_ID)
        .make_state_init()?;
    let cell_builder = CellBuilder::build_from(&state_init)?;
    let hash = cell_builder.repr_hash();
    Ok(StdAddr::new(workchain_id, *hash))
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
    pub public_key: HashBytes,
}

impl InitData {
    pub fn public_key(&self) -> &[u8; 32] {
        &self.public_key.0
    }

    pub fn from_key(key: &VerifyingKey) -> Self {
        Self {
            seqno: 0,
            wallet_id: 0,
            public_key: HashBytes(key.to_bytes()),
        }
    }

    pub fn with_wallet_id(mut self, id: u32) -> Self {
        self.wallet_id = id;
        self
    }

    pub fn compute_addr(&self, workchain_id: i8) -> Result<StdAddr> {
        let state_init = self.make_state_init()?;
        let cell_builder = CellBuilder::build_from(&state_init)?;
        let hash = cell_builder.repr_hash();
        Ok(StdAddr::new(workchain_id, *hash))
    }

    pub fn make_state_init(&self) -> Result<StateInit> {
        Ok(StateInit {
            code: Some(wallet_v3()),
            data: Some(self.serialize()?),
            ..Default::default()
        })
    }

    pub fn serialize(&self) -> Result<Cell> {
        let mut builder = CellBuilder::new();
        builder.store_u32(self.seqno)?;
        builder.store_u32(self.wallet_id)?;
        builder.store_u256(&self.public_key)?;
        let data = builder.build()?;
        Ok(data)
    }

    pub fn make_transfer_payload(
        &self,
        gifts: impl IntoIterator<Item = Gift>,
        expire_at: u32,
    ) -> Result<(HashBytes, CellBuilder)> {
        // insert prefix
        let mut builder = CellBuilder::new();
        builder.store_u32(self.wallet_id)?;
        builder.store_u32(expire_at)?;
        builder.store_u32(self.seqno)?;

        // create internal message
        for gift in gifts {
            let internal_message = OwnedRelaxedMessage {
                info: RelaxedMsgInfo::Int(RelaxedIntMsgInfo {
                    ihr_disabled: true,
                    bounce: gift.bounce,
                    dst: IntAddr::Std(gift.destination),
                    value: CurrencyCollection::new(gift.amount),
                    ..Default::default()
                }),
                init: gift.state_init,
                body: gift.body.unwrap_or(Default::default()).into(),
                layout: None,
            };
            // append it to the body
            builder.store_u8(gift.flags)?;
            builder.store_reference(CellBuilder::build_from(internal_message)?)?;
        }

        let payload = builder.clone().build()?;
        let hash = payload.repr_hash();

        Ok((*hash, builder))
    }
}

impl TryFrom<&Cell> for InitData {
    type Error = anyhow::Error;

    fn try_from(data: &Cell) -> Result<Self, Self::Error> {
        let mut slice = data.as_slice()?;
        let seqno = slice.load_u32()?;
        let wallet_id = slice.load_u32()?;
        let mut buffer = [0u8; 32];
        slice.load_raw(&mut buffer, 32)?;
        let public_key = HashBytes::from_slice(&buffer);

        Ok(Self {
            seqno,
            wallet_id,
            public_key,
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
    #[error("Account address is not valid")]
    InvalidAddress,
}
