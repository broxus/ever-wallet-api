use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::account_enums::{
    AccountStatus, AccountType, AddressResponse, TonEventStatus, TonStatus,
    TonTransactionDirection, TonTransactionStatus,
};
use crate::models::service_id::ServiceId;
use crate::models::sqlx::{TransactionDb, TransactionEventDb};

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("MarkEventsResponse")]
pub struct MarkEventsResponse {
    pub status: TonStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TonEventsResponse")]
pub struct TonEventsResponse {
    pub status: TonStatus,
    pub data: Option<EventsResponse>,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("EventsResponse")]
pub struct EventsResponse {
    pub count: i32,
    pub items: Vec<AccountTransactionEventResponse>,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountTransactionEventResponse")]
pub struct AccountTransactionEventResponse {
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
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub created_at: i64,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub updated_at: i64,
}

impl From<TransactionEventDb> for AccountTransactionEventResponse {
    fn from(c: TransactionEventDb) -> Self {
        AccountTransactionEventResponse {
            id: c.id,
            service_id: c.service_id,
            transaction_id: c.transaction_id,
            message_hash: c.message_hash,
            account_workchain_id: c.account_workchain_id,
            account_hex: c.account_hex,
            balance_change: c.balance_change,
            transaction_direction: c.transaction_direction,
            transaction_status: c.transaction_status,
            event_status: c.event_status,
            created_at: c.created_at.timestamp_millis(),
            updated_at: c.updated_at.timestamp_millis(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountAddressResponse")]
pub struct AccountAddressResponse {
    pub status: TonStatus,
    pub data: Option<AddressResponse>,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountTransactionResponse")]
pub struct AccountTransactionResponse {
    pub status: TonStatus,
    pub data: Option<AccountTransactionDataResponse>,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountTransactionDataResponse")]
pub struct AccountTransactionDataResponse {
    pub id: Uuid,
    pub message_hash: String,
    pub transaction_hash: Option<String>,
    pub transaction_lt: Option<String>,
    pub account: AddressResponse,
    pub value: Option<BigDecimal>,
    pub balance_change: BigDecimal,
    pub direction: TonTransactionDirection,
    pub status: TonTransactionStatus,
    pub aborted: bool,
    pub bounce: bool,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub created_at: i64,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub updated_at: i64,
}

impl From<TransactionDb> for AccountTransactionDataResponse {
    fn from(c: TransactionDb) -> Self {
        AccountTransactionDataResponse {
            id: c.id,
            message_hash: c.message_hash,
            transaction_hash: c.transaction_hash,
            transaction_lt: c.transaction_lt.map(|v| v.to_string()),
            account: c.account,
            value: c.value,
            balance_change: c.balance_change.unwrap_or_default(),
            direction: c.direction,
            status: c.status,
            aborted: c.aborted,
            bounce: c.bounce,
            created_at: c.created_at.timestamp_millis(),
            updated_at: c.updated_at.timestamp_millis(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostAddressValidResponse")]
pub struct PostAddressValidResponse {
    pub valid: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostCheckedAddressResponse")]
pub struct PostCheckedAddressResponse {
    pub status: TonStatus,
    pub data: Option<PostAddressValidResponse>,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostAddressBalanceResponse")]
pub struct PostAddressBalanceResponse {
    pub status: TonStatus,
    pub data: Option<PostAddressBalanceDataResponse>,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostAddressBalanceDataResponse")]
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
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub created_at: i64,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub updated_at: i64,
}
