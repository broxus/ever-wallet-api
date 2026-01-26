use std::convert::TryFrom;

use anyhow::Result;
use ed25519_dalek::PublicKey;
use ton_block::{MsgAddrStd, MsgAddressInt, Serializable};
use ton_types::{BuilderData, Cell, IBitstring, SliceData, UInt256};

use nekoton_utils::*;

use super::{Gift, TonWalletDetails, TransferAction};
use crate::core::models::{Expiration, ExpireAt};
use crate::crypto::{SignedMessage, UnsignedMessage};

pub fn prepare_deploy(
    clock: &dyn Clock,
    public_key: &PublicKey,
    workchain: i8,
    expiration: Expiration,
    version: WalletVersion,
) -> Result<Box<dyn UnsignedMessage>> {
    let init_data = InitData::from_key(public_key).with_subwallet_id(WALLET_ID);
    let dst = compute_contract_address(public_key, workchain, version);
    let mut message =
        ton_block::Message::with_ext_in_header(ton_block::ExternalInboundMessageHeader {
            dst,
            ..Default::default()
        });

    message.set_state_init(init_data.make_state_init(version)?);

    let expire_at = ExpireAt::new(clock, expiration);
    let (hash, payload) = init_data.make_transfer_payload(None, expire_at.timestamp, version)?;

    Ok(Box::new(UnsignedWallet {
        init_data,
        gifts: Vec::new(),
        payload,
        message,
        expire_at,
        hash,
        version,
    }))
}

pub fn prepare_state_init(
    public_key: &PublicKey,
    version: WalletVersion,
) -> Result<ton_block::StateInit> {
    let init_data = InitData::from_key(public_key).with_subwallet_id(WALLET_ID);
    init_data.make_state_init(version)
}

pub fn prepare_transfer(
    clock: &dyn Clock,
    public_key: &PublicKey,
    current_state: &ton_block::AccountStuff,
    seqno_offset: u32,
    gifts: Vec<Gift>,
    expiration: Expiration,
    version: WalletVersion,
) -> Result<TransferAction> {
    if gifts.len() > MAX_MESSAGES {
        return Err(WalletV4Error::TooManyGifts.into());
    }

    let (mut init_data, with_state_init) = match &current_state.storage.state {
        ton_block::AccountState::AccountActive { state_init, .. } => match &state_init.data {
            Some(data) => (InitData::try_from(data)?, false),
            None => return Err(WalletV4Error::InvalidInitData.into()),
        },
        ton_block::AccountState::AccountFrozen { .. } => {
            return Err(WalletV4Error::AccountIsFrozen.into())
        }
        ton_block::AccountState::AccountUninit => (
            InitData::from_key(public_key).with_subwallet_id(WALLET_ID),
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
        message.set_state_init(init_data.make_state_init(version)?);
    }

    let expire_at = ExpireAt::new(clock, expiration);
    let (hash, payload) =
        init_data.make_transfer_payload(gifts.clone(), expire_at.timestamp, version)?;

    Ok(TransferAction::Sign(Box::new(UnsignedWallet {
        init_data,
        gifts,
        payload,
        hash,
        expire_at,
        message,
        version,
    })))
}

#[derive(Clone)]
struct UnsignedWallet {
    init_data: InitData,
    gifts: Vec<Gift>,
    payload: BuilderData,
    hash: UInt256,
    expire_at: ExpireAt,
    message: ton_block::Message,
    version: WalletVersion,
}

impl UnsignedMessage for UnsignedWallet {
    fn refresh_timeout(&mut self, clock: &dyn Clock) {
        if !self.expire_at.refresh(clock) {
            return;
        }

        let (hash, payload) = self
            .init_data
            .make_transfer_payload(self.gifts.clone(), self.expire_at(), self.version)
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
        payload.append_raw(signature, signature.len() * 8)?;
        let body = payload.into_cell()?;

        let mut message = self.message.clone();
        message.set_body(prune_deep_cells(&body, prune_after_depth)?);

        Ok(SignedMessage {
            message,
            expire_at: self.expire_at(),
        })
    }
}

pub static CODE_HASH_V3_R1: &[u8; 32] = &[
    0xB6, 0x10, 0x41, 0xA5, 0x8A, 0x79, 0x80, 0xB9, 0x46, 0xE8, 0xFB, 0x9E, 0x19, 0x8E, 0x3C, 0x90,
    0x4D, 0x24, 0x79, 0x9F, 0xFA, 0x36, 0x57, 0x4E, 0xA4, 0x25, 0x1C, 0x41, 0xA5, 0x66, 0xF5, 0x81,
];

pub static CODE_HASH_V3_R2: &[u8; 32] = &[
    0x84, 0xDA, 0xFA, 0x44, 0x9F, 0x98, 0xA6, 0x98, 0x77, 0x89, 0xBA, 0x23, 0x23, 0x58, 0x07, 0x2B,
    0xC0, 0xF7, 0x6D, 0xC4, 0x52, 0x40, 0x02, 0xA5, 0xD0, 0x91, 0x8B, 0x9A, 0x75, 0xD2, 0xD5, 0x99,
];

pub static CODE_HASH_V4_R1: &[u8; 32] = &[
    0x64, 0xDD, 0x54, 0x80, 0x55, 0x22, 0xC5, 0xBE, 0x8A, 0x9D, 0xB5, 0x9C, 0xEA, 0x01, 0x05, 0xCC,
    0xF0, 0xD0, 0x87, 0x86, 0xCA, 0x79, 0xBE, 0xB8, 0xCB, 0x79, 0xE8, 0x80, 0xA8, 0xD7, 0x32, 0x2D,
];

pub static CODE_HASH_V4_R2: &[u8; 32] = &[
    0xFE, 0xB5, 0xFF, 0x68, 0x20, 0xE2, 0xFF, 0x0D, 0x94, 0x83, 0xE7, 0xE0, 0xD6, 0x2C, 0x81, 0x7D,
    0x84, 0x67, 0x89, 0xFB, 0x4A, 0xE5, 0x80, 0xC8, 0x78, 0x86, 0x6D, 0x95, 0x9D, 0xAB, 0xD5, 0xC0,
];

pub fn is_wallet_v3r1(code_hash: &UInt256) -> bool {
    code_hash.as_slice() == CODE_HASH_V3_R1
}

pub fn is_wallet_v3r2(code_hash: &UInt256) -> bool {
    code_hash.as_slice() == CODE_HASH_V3_R2
}

pub fn is_wallet_v4r1(code_hash: &UInt256) -> bool {
    code_hash.as_slice() == CODE_HASH_V4_R1
}

pub fn is_wallet_v4r2(code_hash: &UInt256) -> bool {
    code_hash.as_slice() == CODE_HASH_V4_R2
}

pub fn compute_contract_address(
    public_key: &PublicKey,
    workchain_id: i8,
    version: WalletVersion,
) -> MsgAddressInt {
    InitData::from_key(public_key)
        .with_subwallet_id(WALLET_ID)
        .compute_addr(workchain_id, version)
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

/// `Default Wallet` init data
#[derive(Clone, Copy)]
pub struct InitData {
    pub seqno: u32,
    pub wallet_id: i32,
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

    pub fn with_subwallet_id(mut self, id: i32) -> Self {
        self.wallet_id = id;
        self
    }

    pub fn compute_addr(&self, workchain_id: i8, version: WalletVersion) -> Result<MsgAddressInt> {
        let init_state = self.make_state_init(version)?.serialize()?;
        let hash = init_state.repr_hash();
        Ok(MsgAddressInt::AddrStd(MsgAddrStd {
            anycast: None,
            workchain_id,
            address: hash.into(),
        }))
    }

    pub fn make_state_init(&self, version: WalletVersion) -> Result<ton_block::StateInit> {
        let code = match version {
            WalletVersion::V3R1 => nekoton_contracts::wallets::code::wallet_v3r1(),
            WalletVersion::V3R2 => nekoton_contracts::wallets::code::wallet_v3r2(),
            WalletVersion::V4R1 => nekoton_contracts::wallets::code::wallet_v4r1(),
            WalletVersion::V4R2 => nekoton_contracts::wallets::code::wallet_v4r2(),
        };

        Ok(ton_block::StateInit {
            code: Some(code),
            data: Some(self.serialize(version)?),
            ..Default::default()
        })
    }

    pub fn serialize(&self, version: WalletVersion) -> Result<Cell> {
        let mut data = BuilderData::new();
        data.append_u32(self.seqno)?
            .append_i32(self.wallet_id)?
            .append_raw(self.public_key.as_slice(), 256)?;

        if matches!(version, WalletVersion::V4R1 | WalletVersion::V4R2) {
            // empty plugin dict
            data.append_bit_zero()?;
        }

        data.into_cell()
    }

    pub fn make_transfer_payload(
        &self,
        gifts: impl IntoIterator<Item = Gift>,
        expire_at: u32,
        version: WalletVersion,
    ) -> Result<(UInt256, BuilderData)> {
        let mut payload = BuilderData::new();

        // insert prefix
        payload
            .append_i32(self.wallet_id)?
            .append_u32(expire_at)?
            .append_u32(self.seqno)?;

        // Opcode
        if matches!(version, WalletVersion::V4R1 | WalletVersion::V4R2) {
            payload.append_u8(0)?;
        }

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
            wallet_id: cs.get_next_i32()?,
            public_key: UInt256::from_be_bytes(&cs.get_next_bytes(32)?),
        })
    }
}

const WALLET_ID: i32 = 0x29A9A317;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WalletVersion {
    V3R1,
    V3R2,
    V4R1,
    V4R2,
}

#[derive(thiserror::Error, Debug)]
enum WalletV4Error {
    #[error("Invalid init data")]
    InvalidInitData,
    #[error("Account is frozen")]
    AccountIsFrozen,
    #[error("Too many outgoing messages")]
    TooManyGifts,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use ton_block::Deserializable;
    use ton_types::UInt256;

    use nekoton_contracts::wallets;

    use crate::core::ton_wallet::wallet_v3v4::{
        is_wallet_v4r1, is_wallet_v4r2, InitData, WalletVersion, WALLET_ID,
    };

    #[test]
    fn code_hash_v4r1() -> anyhow::Result<()> {
        let code_cell = wallets::code::wallet_v4r1();

        let is_wallet_v4r1 = is_wallet_v4r1(&code_cell.repr_hash());
        assert!(is_wallet_v4r1);

        Ok(())
    }

    #[test]
    fn code_hash_v4r2() -> anyhow::Result<()> {
        let code_cell = wallets::code::wallet_v4r2();

        let is_wallet_v4r2 = is_wallet_v4r2(&code_cell.repr_hash());
        assert!(is_wallet_v4r2);

        Ok(())
    }

    #[test]
    fn state_init_v4r2() -> anyhow::Result<()> {
        let state_init_base64 = "te6ccgECFgEAAwQAAgE0AQIBFP8A9KQT9LzyyAsDAFEAAAAAKamjF2dW1vNw/It5bDWN3jVo5dxzZVk+Q11lVLs3LamPSWAVQAIBIAQFAgFIBgcE+PKDCNcYINMf0x/THwL4I7vyZO1E0NMf0x/T//QE0VFDuvKhUVG68qIF+QFUEGT5EPKj+AAkpMjLH1JAyx9SMMv/UhD0AMntVPgPAdMHIcAAn2xRkyDXSpbTB9QC+wDoMOAhwAHjACHAAuMAAcADkTDjDQOkyMsfEssfy/8SExQVAubQAdDTAyFxsJJfBOAi10nBIJJfBOAC0x8hghBwbHVnvSKCEGRzdHK9sJJfBeAD+kAwIPpEAcjKB8v/ydDtRNCBAUDXIfQEMFyBAQj0Cm+hMbOSXwfgBdM/yCWCEHBsdWe6kjgw4w0DghBkc3RyupJfBuMNCAkCASAKCwB4AfoA9AQw+CdvIjBQCqEhvvLgUIIQcGx1Z4MesXCAGFAEywUmzxZY+gIZ9ADLaRfLH1Jgyz8gyYBA+wAGAIpQBIEBCPRZMO1E0IEBQNcgyAHPFvQAye1UAXKwjiOCEGRzdHKDHrFwgBhQBcsFUAPPFiP6AhPLassfyz/JgED7AJJfA+ICASAMDQBZvSQrb2omhAgKBrkPoCGEcNQICEekk30pkQzmkD6f+YN4EoAbeBAUiYcVnzGEAgFYDg8AEbjJftRNDXCx+AA9sp37UTQgQFA1yH0BDACyMoHy//J0AGBAQj0Cm+hMYAIBIBARABmtznaiaEAga5Drhf/AABmvHfaiaEAQa5DrhY/AAG7SB/oA1NQi+QAFyMoHFcv/ydB3dIAYyMsFywIizxZQBfoCFMtrEszMyXP7AMhAFIEBCPRR8qcCAHCBAQjXGPoA0z/IVCBHgQEI9FHyp4IQbm90ZXB0gBjIywXLAlAGzxZQBPoCFMtqEssfyz/Jc/sAAgBsgQEI1xj6ANM/MFIkgQEI9Fnyp4IQZHN0cnB0gBjIywXLAlAFzxZQA/oCE8tqyx8Syz/Jc/sAAAr0AMntVA==";

        let state_init = ton_block::StateInit::construct_from_base64(state_init_base64)?;

        let init_data_clone = InitData {
            seqno: 0,
            wallet_id: WALLET_ID,
            public_key: UInt256::from_str(
                "6756d6f370fc8b796c358dde3568e5dc7365593e435d6554bb372da98f496015",
            )?,
        };

        let state_init_clone = init_data_clone.make_state_init(WalletVersion::V4R2)?;

        assert_eq!(state_init, state_init_clone);

        Ok(())
    }
}
