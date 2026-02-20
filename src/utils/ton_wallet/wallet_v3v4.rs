use std::convert::TryFrom;

use anyhow::Result;
use ed25519_dalek::VerifyingKey;
use tycho_types::{
    abi::{AbiVersion, UnsignedBody, UnsignedExternalMessage},
    cell::{Cell, CellBuilder, HashBytes},
    models::{
        Account, AccountState, CurrencyCollection, IntAddr, IntMsgInfo, Message, MsgInfo,
        StateInit, StdAddr,
    },
};

use crate::utils::{
    ton_wallet::{Gift, TonWalletDetails},
    wallets::{self},
};

pub fn prepare_deploy(
    public_key: &VerifyingKey,
    workchain: i8,
    expire_at: u32,
    version: WalletVersion,
) -> Result<UnsignedExternalMessage> {
    let init_data = InitData::from_key(public_key).with_subwallet_id(WALLET_ID);
    let dst = compute_contract_address(public_key, workchain, version)?;

    let (hash, payload) = init_data.make_transfer_payload(None, expire_at, version)?;
    let unsigned_body = UnsignedBody {
        payload,
        hash,
        abi_version: AbiVersion::V2_3,
        expire_at,
    };
    let mut unsigned_message = unsigned_body.with_dst(dst);
    let state_init = init_data.make_state_init(version)?;
    unsigned_message.set_state_init(Some(state_init));
    Ok(unsigned_message)
}

pub fn prepare_state_init(public_key: &VerifyingKey, version: WalletVersion) -> Result<StateInit> {
    let init_data = InitData::from_key(public_key).with_subwallet_id(WALLET_ID);
    init_data.make_state_init(version)
}

pub fn prepare_transfer(
    public_key: &VerifyingKey,
    current_state: &Account,
    seqno_offset: u32,
    gifts: Vec<Gift>,
    expire_at: u32,
    version: WalletVersion,
) -> Result<UnsignedExternalMessage> {
    if gifts.len() > MAX_MESSAGES {
        return Err(WalletV4Error::TooManyGifts.into());
    }

    let (mut init_data, with_state_init) = match &current_state.state {
        AccountState::Active(state_init) => match &state_init.data {
            Some(data) => (InitData::try_from(data)?, false),
            None => return Err(WalletV4Error::InvalidInitData.into()),
        },
        AccountState::Frozen { .. } => return Err(WalletV4Error::AccountIsFrozen.into()),
        AccountState::Uninit => (
            InitData::from_key(public_key).with_subwallet_id(WALLET_ID),
            true,
        ),
    };

    init_data.seqno += seqno_offset;

    let (hash, payload) = init_data.make_transfer_payload(gifts.clone(), expire_at, version)?;

    let unsigned_body = UnsignedBody {
        payload,
        hash,
        abi_version: AbiVersion::V2_3,
        expire_at,
    };
    let mut unsigned_message = unsigned_body.with_dst(
        current_state
            .address
            .as_std()
            .ok_or(WalletV4Error::InvalidAddress)?
            .clone(),
    );
    if with_state_init {
        let state_init = init_data.make_state_init(version)?;
        unsigned_message.set_state_init(Some(state_init));
    }

    Ok(unsigned_message)
}

#[allow(unused)]
struct UnsignedWallet {
    init_data: InitData,
    gifts: Vec<Gift>,
    payload: Cell,
    hash: HashBytes,
    expire_at: u32,
    message: UnsignedExternalMessage,
    version: WalletVersion,
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

pub fn is_wallet_v3r1(code_hash: &HashBytes) -> bool {
    code_hash.as_slice() == CODE_HASH_V3_R1
}

pub fn is_wallet_v3r2(code_hash: &HashBytes) -> bool {
    code_hash.as_slice() == CODE_HASH_V3_R2
}

pub fn is_wallet_v4r1(code_hash: &HashBytes) -> bool {
    code_hash.as_slice() == CODE_HASH_V4_R1
}

pub fn is_wallet_v4r2(code_hash: &HashBytes) -> bool {
    code_hash.as_slice() == CODE_HASH_V4_R2
}

pub fn compute_contract_address(
    public_key: &VerifyingKey,
    workchain_id: i8,
    version: WalletVersion,
) -> Result<StdAddr> {
    InitData::from_key(public_key)
        .with_subwallet_id(WALLET_ID)
        .compute_addr(workchain_id, version)
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
            public_key: HashBytes::from_slice(key.as_bytes()),
        }
    }

    pub fn with_subwallet_id(mut self, id: i32) -> Self {
        self.wallet_id = id;
        self
    }

    pub fn compute_addr(&self, workchain_id: i8, version: WalletVersion) -> Result<StdAddr> {
        let state_init = self.make_state_init(version)?;
        let cell_builder = CellBuilder::build_from(&state_init)?;
        let hash = cell_builder.repr_hash();
        Ok(StdAddr::new(workchain_id, *hash))
    }

    pub fn make_state_init(&self, version: WalletVersion) -> Result<StateInit> {
        let code = match version {
            WalletVersion::V3R1 => wallets::code::wallet_v3r1(),
            WalletVersion::V3R2 => wallets::code::wallet_v3r2(),
            WalletVersion::V4R1 => wallets::code::wallet_v4r1(),
            WalletVersion::V4R2 => wallets::code::wallet_v4r2(),
        };

        Ok(StateInit {
            code: Some(code),
            data: Some(self.serialize(version)?),
            ..Default::default()
        })
    }

    pub fn serialize(&self, version: WalletVersion) -> Result<Cell> {
        let mut builder = CellBuilder::new();
        builder.store_u32(self.seqno)?;
        builder.store_u32(self.wallet_id as _)?;
        builder.store_u256(&self.public_key)?;

        if matches!(version, WalletVersion::V4R1 | WalletVersion::V4R2) {
            // empty plugin dict
            builder.store_bit_zero()?;
        }

        let data = builder.build()?;
        Ok(data)
    }

    pub fn make_transfer_payload(
        &self,
        gifts: impl IntoIterator<Item = Gift>,
        expire_at: u32,
        version: WalletVersion,
    ) -> Result<(HashBytes, Cell)> {
        // insert prefix
        let mut builder = CellBuilder::new();
        builder.store_u32(self.wallet_id as _)?;
        builder.store_u32(expire_at)?;
        builder.store_u32(self.seqno)?;

        // Opcode
        if matches!(version, WalletVersion::V4R1 | WalletVersion::V4R2) {
            builder.store_u8(0)?;
        }

        // create internal message
        for gift in gifts {
            let body = gift.body.unwrap_or(Default::default());
            let internal_message = Message {
                info: MsgInfo::Int(IntMsgInfo {
                    ihr_disabled: true,
                    bounce: gift.bounce,
                    dst: IntAddr::Std(gift.destination),
                    value: CurrencyCollection::new(gift.amount),
                    ..Default::default()
                }),
                init: gift.state_init,
                body: body.as_slice()?,
                layout: None,
            };
            // append it to the body
            builder.store_u8(gift.flags)?;
            builder.store_reference(CellBuilder::build_from(internal_message)?)?;
        }

        let payload = builder.build()?;
        let hash = payload.repr_hash();

        Ok((*hash, payload))
    }
}

impl TryFrom<&Cell> for InitData {
    type Error = anyhow::Error;

    fn try_from(data: &Cell) -> Result<Self, Self::Error> {
        let mut slice = data.as_slice()?;

        let seqno = slice.load_u32()?;
        let wallet_id = slice.load_u32()? as i32;
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
    #[error("Account address is not valid")]
    InvalidAddress,
}

#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use tycho_types::{
        boc::Boc,
        cell::{HashBytes, Load},
        models::StateInit,
    };

    use crate::utils::{
        ton_wallet::wallet_v3v4::{
            is_wallet_v4r1, is_wallet_v4r2, InitData, WalletVersion, WALLET_ID,
        },
        wallets,
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
        let state_init = Boc::decode_base64(state_init_base64)?;

        let state_init = StateInit::load_from(&mut state_init.as_slice()?)?;

        let init_data_clone = InitData {
            seqno: 0,
            wallet_id: WALLET_ID,
            public_key: HashBytes::from_str(
                "6756d6f370fc8b796c358dde3568e5dc7365593e435d6554bb372da98f496015",
            )?,
        };

        let state_init_clone = init_data_clone.make_state_init(WalletVersion::V4R2)?;

        assert_eq!(state_init, state_init_clone);

        Ok(())
    }
}
