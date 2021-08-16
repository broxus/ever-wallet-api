use std::str::FromStr;

use nekoton_utils::pack_std_smc_addr;
use serde::{Deserialize, Serialize};
use ton_block::MsgAddressInt;

use crate::models::address::Address;
use crate::models::sqlx::AddressDb;

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, Eq, PartialEq, sqlx::Type)]
#[opg("AccountType")]
#[sqlx(type_name = "twa_account_type", rename_all = "PascalCase")]
pub enum AccountType {
    HighloadWallet,
    Wallet,
    SafeMultisig,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, sqlx::Type)]
#[opg("AccountStatus")]
#[sqlx(type_name = "twa_account_status", rename_all = "PascalCase")]
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

impl From<AddressDb> for AddressResponse {
    fn from(a: AddressDb) -> Self {
        let account = MsgAddressInt::from_str(&format!("{}:{}", a.workchain_id, a.hex)).unwrap();
        let base64url = Address(pack_std_smc_addr(true, &account, false).unwrap());
        Self {
            workchain_id: a.workchain_id,
            hex: Address(a.hex),
            base64url,
        }
    }
}
//
// #[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, PartialEq, Eq, sqlx::Type)]
// #[opg("TonTransactionType")]
// #[sqlx(type_name = "twa_account_status", rename_all = "PascalCase")]
// pub enum TonTransactionType {
//     In,
//     Out,
// }

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, PartialEq, Eq)]
#[opg("TonStatus")]
pub enum TonStatus {
    Ok,
    Error,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, PartialEq, Eq, sqlx::Type)]
#[opg("TonTransactionStatus")]
#[sqlx(type_name = "twa_transaction_status", rename_all = "PascalCase")]
pub enum TonTransactionStatus {
    New,
    Done,
    PartiallyDone,
    Error,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, PartialEq, Eq, sqlx::Type)]
#[opg("TonTransactionStatus")]
#[sqlx(type_name = "twa_token_transaction_status", rename_all = "PascalCase")]
pub enum TonTokenTransactionStatus {
    New,
    Done,
    Error,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, PartialEq, Eq, sqlx::Type, Copy)]
#[opg("TonEventStatus")]
#[sqlx(type_name = "twa_transaction_event_status", rename_all = "PascalCase")]
pub enum TonEventStatus {
    New,
    Notified,
    Error,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, PartialEq, Eq, sqlx::Type)]
#[opg("TonTransactionDirection")]
#[sqlx(type_name = "twa_transaction_direction", rename_all = "PascalCase")]
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
