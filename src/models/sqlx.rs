use bigdecimal::BigDecimal;

use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::models::account_enums::{
    AccountType, TonEventStatus, TonTokenTransactionStatus, TonTransactionDirection,
    TonTransactionStatus,
};
use crate::models::service_id::ServiceId;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct ApiServiceDb {
    pub id: uuid::Uuid,
    pub name: String,
    pub created_at: NaiveDateTime,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct ApiServiceKeyDb {
    pub id: Uuid,
    pub service_id: ServiceId,
    pub key: String,
    pub secret: String,
    pub whitelist: serde_json::Value,
    pub created_at: NaiveDateTime,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct ApiServiceCallbackDb {
    pub id: Uuid,
    pub service_id: ServiceId,
    pub callback: String,
    pub created_at: NaiveDateTime,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct AddressDb {
    pub id: Uuid,
    pub service_id: ServiceId,
    pub workchain_id: i32,
    pub hex: String,
    pub base64url: String,
    pub public_key: String,
    pub private_key: String,
    pub account_type: AccountType,
    pub custodians: Option<i32>,
    pub confirmations: Option<i32>,
    pub custodians_public_keys: Option<serde_json::Value>,
    pub balance: BigDecimal,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct TransactionDb {
    pub id: Uuid,
    pub service_id: ServiceId,
    pub message_hash: String,
    pub transaction_hash: Option<String>,
    pub transaction_lt: Option<BigDecimal>,
    pub transaction_timeout: Option<i64>,
    pub transaction_scan_lt: Option<i64>,
    pub sender_workchain_id: Option<i32>,
    pub sender_hex: Option<String>,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub messages: Option<serde_json::Value>,
    pub data: Option<serde_json::Value>,
    pub original_value: Option<BigDecimal>,
    pub original_outputs: Option<serde_json::Value>,
    pub value: Option<BigDecimal>,
    pub fee: Option<BigDecimal>,
    pub balance_change: Option<BigDecimal>,
    pub direction: TonTransactionDirection,
    pub status: TonTransactionStatus,
    pub error: Option<String>,
    pub aborted: bool,
    pub bounce: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct TransactionEventDb {
    pub id: Uuid,
    pub service_id: ServiceId,
    pub transaction_id: Uuid,
    pub message_hash: String,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub balance_change: Option<BigDecimal>,
    pub transaction_direction: TonTransactionDirection,
    pub transaction_status: TonTransactionStatus,
    pub event_status: TonEventStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct TokenBalanceFromDb {
    pub service_id: ServiceId,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub balance: BigDecimal,
    pub root_address: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct TokenTransactionFromDb {
    pub id: Uuid,
    pub service_id: ServiceId,
    pub transaction_hash: Option<String>,
    pub message_hash: String,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub value: BigDecimal,
    pub root_address: String,
    pub payload: Option<Vec<u8>>,
    pub error: Option<String>,
    pub block_hash: Option<String>,
    pub block_time: Option<u32>,
    pub direction: TonTransactionDirection,
    pub status: TonTokenTransactionStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct TokenTransactionToDb {
    pub transaction_hash: String,
    pub message_hash: String,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub value: BigDecimal,
    pub root_address: String,
    pub payload: Option<Vec<u8>>,
    pub error: Option<String>,
    pub block_hash: String,
    pub block_time: u32,
    pub direction: TonTransactionDirection,
    pub status: TonTokenTransactionStatus,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct TokenTransactionEventDb {
    pub id: Uuid,
    pub service_id: ServiceId,
    pub token_transaction_id: Uuid,
    pub message_hash: String,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub value: BigDecimal,
    pub root_address: String,
    pub transaction_direction: TonTransactionDirection,
    pub transaction_status: TonTransactionStatus,
    pub event_status: TonEventStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct TokenOwnerFromDb {
    pub address: String,
    pub owner_account_workchain_id: i32,
    pub owner_account_hex: String,
    pub root_address: String,
    pub created_at: NaiveDateTime,
}
