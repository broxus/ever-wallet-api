use serde::{Deserialize, Serialize};

use crate::models::address::Address;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum AccountType {
    HighloadWallet,
    Wallet,
    SafeMultisig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum AccountStatus {
    Active,
    UnInit,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddressResponse {
    pub workchain_id: i32,
    pub hex: Address,
    pub base64url: Address,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum TonTransactionType {
    In,
    Out,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum TonStatus {
    Ok,
    Error,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum TonTransactionStatus {
    New,
    Done,
    PartiallyDone,
    Error,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum TonEventStatus {
    New,
    Notified,
    Error,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum TonTransactionDirection {
    Send,
    Receive,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AccountAddressType {
    Internal,
    External,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PostTonTransactionSendOutputType {
    Normal,
    AllBalance,
    AllBalanceDeleteNetworkAccount,
}
