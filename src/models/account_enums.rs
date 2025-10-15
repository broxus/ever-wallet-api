use std::str::FromStr;

use nekoton::core::models::TokenWalletVersion;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;
use tycho_types::models::{AccountState, StdAddr};

use crate::models::{Address, AddressDb};

#[derive(
    Debug, Default, Deserialize, Serialize, Clone, JsonSchema, Eq, PartialEq, sqlx::Type, Copy,
)]
#[sqlx(type_name = "twa_account_type", rename_all = "PascalCase")]
pub enum AccountType {
    #[default]
    HighloadWallet,
    Wallet,
    SafeMultisig,
    EverWallet,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, sqlx::Type, Eq, PartialEq)]
#[sqlx(type_name = "twa_account_status", rename_all = "PascalCase")]
pub enum AccountStatus {
    Active,
    UnInit,
    Frozen,
}

impl From<AccountState> for AccountStatus {
    fn from(state: AccountState) -> Self {
        match state {
            AccountState::Uninit => AccountStatus::UnInit,
            AccountState::Active(_) => AccountStatus::Active,
            AccountState::Frozen(_) => AccountStatus::Frozen,
        }
    }
}

impl From<ton_block::AccountState> for AccountStatus {
    fn from(state: ton_block::AccountState) -> Self {
        match state {
            ton_block::AccountState::AccountUninit => AccountStatus::UnInit,
            ton_block::AccountState::AccountActive { .. } => AccountStatus::Active,
            ton_block::AccountState::AccountFrozen { .. } => AccountStatus::Frozen,
        }
    }
}

#[derive(
    Debug, Deserialize, Serialize, Clone, JsonSchema, Eq, PartialEq, sqlx::Type, Copy, EnumString,
)]
#[sqlx(type_name = "twa_token_wallet_version", rename_all = "PascalCase")]
pub enum TokenWalletVersionDb {
    OldTip3v4,
    Tip3,
}

impl From<TokenWalletVersion> for TokenWalletVersionDb {
    fn from(version: TokenWalletVersion) -> Self {
        match version {
            TokenWalletVersion::OldTip3v4 => TokenWalletVersionDb::OldTip3v4,
            TokenWalletVersion::Tip3 => TokenWalletVersionDb::Tip3,
        }
    }
}

impl From<TokenWalletVersionDb> for TokenWalletVersion {
    fn from(version: TokenWalletVersionDb) -> Self {
        match version {
            TokenWalletVersionDb::OldTip3v4 => TokenWalletVersion::OldTip3v4,
            TokenWalletVersionDb::Tip3 => TokenWalletVersion::Tip3,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub workchain_id: i32,
    pub hex: Address,
    pub base64url: Address,
}

impl From<AddressDb> for Account {
    fn from(a: AddressDb) -> Self {
        let account = StdAddr::from_str(&format!("{}:{}", a.workchain_id, a.hex)).unwrap();
        let base64url = Address(account.display_base64_url(true).to_string());
        Self {
            workchain_id: a.workchain_id,
            hex: Address(a.hex),
            base64url,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq, Eq)]
pub enum TonStatus {
    Ok,
    Error,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "twa_transaction_status", rename_all = "PascalCase")]
pub enum TonTransactionStatus {
    New,
    Done,
    PartiallyDone,
    Error,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "twa_token_transaction_status", rename_all = "PascalCase")]
pub enum TonTokenTransactionStatus {
    New,
    Done,
    Error,
}

impl From<TonTokenTransactionStatus> for TonTransactionStatus {
    fn from(t: TonTokenTransactionStatus) -> Self {
        match t {
            TonTokenTransactionStatus::New => Self::New,
            TonTokenTransactionStatus::Done => Self::Done,
            TonTokenTransactionStatus::Error => Self::Error,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq, Eq, sqlx::Type, Copy)]
#[sqlx(type_name = "twa_transaction_event_status", rename_all = "PascalCase")]
pub enum TonEventStatus {
    New,
    Notified,
    Error,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "twa_transaction_direction", rename_all = "PascalCase")]
pub enum TonTransactionDirection {
    Send,
    Receive,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AccountAddressType {
    Internal,
    External,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
pub enum TransactionsSearchOrdering {
    CreatedAtAsc,
    CreatedAtDesc,
    TransactionLtAsc,
    TransactionLtDesc,
    TransactionTimestampAsc,
    TransactionTimestampDesc,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, Eq, PartialEq, JsonSchema)]
pub enum TransactionSendOutputType {
    #[default]
    Normal,
    AllBalance,
    AllBalanceDeleteNetworkAccount,
}

impl TryFrom<u8> for TransactionSendOutputType {
    type Error = TransactionSendOutputTypeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            3 => Ok(TransactionSendOutputType::Normal),
            128 => Ok(TransactionSendOutputType::AllBalance),
            160 => Ok(TransactionSendOutputType::AllBalanceDeleteNetworkAccount),
            _ => Err(TransactionSendOutputTypeError::UnsupportedMessageFlags),
        }
    }
}

impl From<TransactionSendOutputType> for u8 {
    fn from(value: TransactionSendOutputType) -> u8 {
        match value {
            TransactionSendOutputType::Normal => 3,
            TransactionSendOutputType::AllBalance => 128,
            TransactionSendOutputType::AllBalanceDeleteNetworkAccount => 128 + 32,
        }
    }
}

#[derive(thiserror::Error, Debug, Copy, Clone)]
pub enum TransactionSendOutputTypeError {
    #[error("Unsupported message flags set")]
    UnsupportedMessageFlags,
}
