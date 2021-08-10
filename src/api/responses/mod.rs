use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::account_enums::{
    AccountStatus, AccountType, AddressResponse, TonStatus, TonTransactionDirection,
    TonTransactionStatus,
};
use crate::models::account_transaction_event::AccountTransactionEvent;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarkEventsResponse {
    pub status: TonStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TonEventsResponse {
    pub status: TonStatus,
    pub data: Option<EventsResponse>,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EventsResponse {
    pub count: i32,
    pub items: Vec<AccountTransactionEvent>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountAddressResponse {
    pub status: TonStatus,
    pub data: Option<AddressResponse>,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountTransactionResponse {
    pub status: TonStatus,
    pub data: Option<AccountTransactionDataResponse>,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountTransactionDataResponse {
    pub id: Uuid,
    pub message_hash: TxHash,
    pub transaction_hash: Option<TxHash>,
    pub transaction_lt: Option<String>,
    pub account: AddressResponse,
    pub value: Option<BigDecimal>,
    pub balance_change: BigDecimal,
    pub direction: TonTransactionDirection,
    pub status: TonTransactionStatus,
    pub aborted: bool,
    pub bounce: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PostAddressValidResponse {
    pub valid: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PostCheckedAddressResponse {
    pub status: TonStatus,
    pub data: Option<PostAddressValidResponse>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PostAddressBalanceResponse {
    pub status: TonStatus,
    pub data: Option<PostAddressBalanceDataResponse>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PostAddressBalanceDataResponse {
    pub id: Uuid,
    pub address: AddressResponse,
    pub account_type: AccountType,
    pub account_status: AccountStatus,
    pub balance: BigDecimal,
    pub network_balance: BigDecimal,
    pub last_transaction_hash: Option<String>,
    pub last_transaction_lt: Option<String>,
    pub sync_u_time: i64,
    pub created_at: i64,
    pub updated_at: i64,
}
