use std::num::NonZeroU8;
use std::str::FromStr;

use anyhow::Result;
use ed25519_dalek::PublicKey;
use serde::{Deserialize, Serialize};

use nekoton_utils::*;
use tycho_types::cell::{Cell, HashBytes};
use tycho_types::models::{IntAddr, StateInit, StdAddr};

pub use self::multisig::MultisigType;

pub mod ever_wallet;
pub mod highload_wallet_v2;
pub mod multisig;
pub mod wallet_v3;
pub mod wallet_v3v4;
pub mod wallet_v5r1;

pub const DEFAULT_WORKCHAIN: i8 = 0;

pub const WALLET_TYPES_BY_POPULARITY: [WalletType; 10] = [
    WalletType::Multisig(MultisigType::SafeMultisigWallet),
    WalletType::Multisig(MultisigType::SurfWallet),
    WalletType::WalletV3,
    WalletType::EverWallet,
    WalletType::Multisig(MultisigType::Multisig2_1),
    WalletType::Multisig(MultisigType::Multisig2),
    WalletType::Multisig(MultisigType::SetcodeMultisigWallet),
    WalletType::Multisig(MultisigType::SafeMultisigWallet24h),
    WalletType::Multisig(MultisigType::BridgeMultisigWallet),
    WalletType::HighloadWalletV2,
];

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TonWalletDetails {
    pub requires_separate_deploy: bool,
    #[serde(with = "serde_string")]
    pub min_amount: u64,
    pub max_messages: usize,
    pub supports_payload: bool,
    pub supports_state_init: bool,
    pub supports_multiple_owners: bool,
    pub supports_code_update: bool,
    pub expiration_time: u32,
    pub required_confirmations: Option<NonZeroU8>,
}

/// Message info
#[derive(Clone)]
pub struct Gift {
    pub flags: u8,
    pub bounce: bool,
    pub destination: IntAddr,
    pub amount: u128,
    pub body: Option<Cell>,
    pub state_init: Option<StateInit>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum WalletType {
    Multisig(MultisigType),
    WalletV3,
    WalletV3R1,
    WalletV3R2,
    WalletV4R1,
    WalletV4R2,
    WalletV5R1,
    HighloadWalletV2,
    EverWallet,
}

impl WalletType {
    pub fn details(&self) -> TonWalletDetails {
        match self {
            Self::Multisig(multisig_type) => multisig::ton_wallet_details(*multisig_type),
            Self::WalletV3 => wallet_v3::DETAILS,
            Self::WalletV5R1 => wallet_v5r1::DETAILS,
            Self::HighloadWalletV2 => highload_wallet_v2::DETAILS,
            Self::EverWallet => ever_wallet::DETAILS,
            Self::WalletV3R1 | Self::WalletV3R2 | Self::WalletV4R1 | Self::WalletV4R2 => {
                wallet_v3v4::DETAILS
            }
        }
    }

    pub fn possible_updates(&self) -> &'static [Self] {
        const MULTISIG2_UPDATES: &[WalletType] = &[WalletType::Multisig(MultisigType::Multisig2_1)];

        match self {
            Self::Multisig(MultisigType::Multisig2) => MULTISIG2_UPDATES,
            _ => &[],
        }
    }

    pub fn code_hash(&self) -> &[u8; 32] {
        match self {
            Self::Multisig(multisig_type) => multisig_type.code_hash(),
            Self::WalletV3 => wallet_v3::CODE_HASH,
            Self::WalletV3R1 => wallet_v3v4::CODE_HASH_V3_R1,
            Self::WalletV3R2 => wallet_v3v4::CODE_HASH_V3_R2,
            Self::WalletV4R1 => wallet_v3v4::CODE_HASH_V4_R1,
            Self::WalletV4R2 => wallet_v3v4::CODE_HASH_V4_R2,
            Self::WalletV5R1 => wallet_v5r1::CODE_HASH,
            Self::HighloadWalletV2 => highload_wallet_v2::CODE_HASH,
            Self::EverWallet => ever_wallet::CODE_HASH,
        }
    }

    pub fn code(&self) -> Cell {
        use nekoton_contracts::wallets;
        match self {
            Self::Multisig(multisig_type) => multisig_type.code(),
            Self::WalletV3 => wallets::code::wallet_v3(),
            Self::WalletV3R1 => wallets::code::wallet_v3r1(),
            Self::WalletV3R2 => wallets::code::wallet_v3r2(),
            Self::WalletV4R1 => wallets::code::wallet_v4r1(),
            Self::WalletV4R2 => wallets::code::wallet_v4r2(),
            Self::WalletV5R1 => wallets::code::wallet_v5r1(),
            Self::HighloadWalletV2 => wallets::code::highload_wallet_v2(),
            Self::EverWallet => wallets::code::ever_wallet(),
        }
    }
}

impl FromStr for WalletType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "WalletV3" => Self::WalletV3,
            "WalletV3R1" => Self::WalletV3R1,
            "WalletV3R2" => Self::WalletV3R2,
            "WalletV4R1" => Self::WalletV4R1,
            "WalletV4R2" => Self::WalletV4R2,
            "WalletV5R1" => Self::WalletV5R1,
            "HighloadWalletV2" => Self::HighloadWalletV2,
            "EverWallet" => Self::EverWallet,
            s => Self::Multisig(MultisigType::from_str(s)?),
        })
    }
}

impl TryInto<u16> for WalletType {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<u16, Self::Error> {
        let res = match self {
            WalletType::WalletV3 => 0,
            WalletType::EverWallet => 1,
            WalletType::Multisig(MultisigType::SafeMultisigWallet) => 2,
            WalletType::Multisig(MultisigType::SafeMultisigWallet24h) => 3,
            WalletType::Multisig(MultisigType::SetcodeMultisigWallet) => 4,
            WalletType::Multisig(MultisigType::BridgeMultisigWallet) => 5,
            WalletType::Multisig(MultisigType::SurfWallet) => 6,
            WalletType::Multisig(MultisigType::Multisig2) => 7,
            WalletType::Multisig(MultisigType::Multisig2_1) => 8,
            WalletType::WalletV4R1 => 9,
            WalletType::WalletV4R2 => 10,
            WalletType::WalletV5R1 => 11,
            _ => anyhow::bail!("Unimplemented wallet type"),
        };

        Ok(res)
    }
}

impl std::fmt::Display for WalletType {
    fn fmt(&self, f: &'_ mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Multisig(multisig_type) => multisig_type.fmt(f),
            Self::WalletV3 => f.write_str("WalletV3"),
            Self::WalletV3R1 => f.write_str("WalletV3R1"),
            Self::WalletV3R2 => f.write_str("WalletV3R2"),
            Self::WalletV4R1 => f.write_str("WalletV4R1"),
            Self::WalletV4R2 => f.write_str("WalletV4R2"),
            Self::WalletV5R1 => f.write_str("WalletV5R1"),
            Self::HighloadWalletV2 => f.write_str("HighloadWalletV2"),
            Self::EverWallet => f.write_str("EverWallet"),
        }
    }
}

pub fn compute_address(
    public_key: &PublicKey,
    wallet_type: WalletType,
    workchain_id: i8,
) -> Result<StdAddr> {
    match wallet_type {
        WalletType::Multisig(multisig_type) => {
            multisig::compute_contract_address(public_key, multisig_type, workchain_id)
        }
        WalletType::WalletV3 => wallet_v3::compute_contract_address(public_key, workchain_id),
        WalletType::WalletV5R1 => wallet_v5r1::compute_contract_address(public_key, workchain_id),
        WalletType::EverWallet => ever_wallet::compute_contract_address(public_key, workchain_id),
        WalletType::HighloadWalletV2 => {
            highload_wallet_v2::compute_contract_address(public_key, workchain_id)
        }
        WalletType::WalletV3R1 => wallet_v3v4::compute_contract_address(
            public_key,
            workchain_id,
            wallet_v3v4::WalletVersion::V3R1,
        ),
        WalletType::WalletV3R2 => wallet_v3v4::compute_contract_address(
            public_key,
            workchain_id,
            wallet_v3v4::WalletVersion::V3R2,
        ),
        WalletType::WalletV4R1 => wallet_v3v4::compute_contract_address(
            public_key,
            workchain_id,
            wallet_v3v4::WalletVersion::V4R1,
        ),
        WalletType::WalletV4R2 => wallet_v3v4::compute_contract_address(
            public_key,
            workchain_id,
            wallet_v3v4::WalletVersion::V4R2,
        ),
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MultisigPendingTransaction {
    pub id: u64,
    pub confirmations: Vec<HashBytes>,
    pub signs_required: u8,
    pub signs_received: u8,
    pub creator: HashBytes,
    pub index: u8,
    pub dest: StdAddr,
    pub value: u128,
    pub send_flags: u16,
    pub payload: Cell,
    pub bounce: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MultisigPendingUpdate {
    pub id: u64,
    pub confirmations: Vec<HashBytes>,
    pub signs_received: u8,
    pub creator: HashBytes,
    pub index: u8,
    pub new_code_hash: Option<HashBytes>,
    pub new_custodians: Option<Vec<HashBytes>>,
    pub new_req_confirms: Option<u8>,
    pub new_lifetime: Option<u32>,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub enum MessageFlags {
    #[default]
    Normal,
    AllBalance,
    AllBalanceDeleteNetworkAccount,
}

impl TryFrom<u8> for MessageFlags {
    type Error = MessageFlagsError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            3 => Ok(MessageFlags::Normal),
            128 => Ok(MessageFlags::AllBalance),
            160 => Ok(MessageFlags::AllBalanceDeleteNetworkAccount),
            _ => Err(MessageFlagsError::UnknownMessageFlags),
        }
    }
}

impl From<MessageFlags> for u8 {
    fn from(value: MessageFlags) -> u8 {
        match value {
            MessageFlags::Normal => 3,
            MessageFlags::AllBalance => 128,
            MessageFlags::AllBalanceDeleteNetworkAccount => 128 + 32,
        }
    }
}

#[derive(thiserror::Error, Debug, Copy, Clone)]
pub enum MessageFlagsError {
    #[error("Unknown message flags combination")]
    UnknownMessageFlags,
}
