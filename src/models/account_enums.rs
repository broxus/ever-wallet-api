use serde::{Deserialize, Serialize};

use crate::models::address::Address;

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[opg("AccountType")]
pub enum AccountType {
    HighloadWallet,
    Wallet,
    SafeMultisig,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[opg("AccountStatus")]
pub enum AccountStatus {
    Active,
    UnInit,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AddressResponse")]
pub struct AddressResponse {
    pub workchain_id: i32,
    pub hex: Address,
    pub base64url: Address,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, PartialEq, Eq)]
#[opg("TonTransactionType")]
pub enum TonTransactionType {
    In,
    Out,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, PartialEq, Eq)]
#[opg("TonStatus")]
pub enum TonStatus {
    Ok,
    Error,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, PartialEq, Eq)]
#[opg("TonTransactionStatus")]
pub enum TonTransactionStatus {
    New,
    Done,
    PartiallyDone,
    Error,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, PartialEq, Eq)]
#[opg("TonEventStatus")]
pub enum TonEventStatus {
    New,
    Notified,
    Error,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, PartialEq, Eq)]
#[opg("TonTransactionDirection")]
pub enum TonTransactionDirection {
    Send,
    Receive,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "lowercase")]
#[opg("AccountAddressType")]
pub enum AccountAddressType {
    Internal,
    External,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[opg("TransactionSendOutputType")]
pub enum TransactionSendOutputType {
    Normal,
    AllBalance,
    AllBalanceDeleteNetworkAccount,
}
