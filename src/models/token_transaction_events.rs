use bigdecimal::BigDecimal;
use uuid::Uuid;

use crate::models::account_enums::{
    TonEventStatus, TonTokenTransactionStatus, TonTransactionDirection,
};
use crate::models::service_id::ServiceId;
use crate::models::sqlx::TokenTransactionFromDb;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct CreateSendTokenTransactionEvent {
    pub id: Uuid,
    pub service_id: ServiceId,
    pub token_transaction_id: Uuid,
    pub message_hash: String,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub value: BigDecimal,
    pub root_address: String,
    pub transaction_direction: TonTransactionDirection,
    pub transaction_status: TonTokenTransactionStatus,
    pub event_status: TonEventStatus,
}

impl CreateSendTokenTransactionEvent {
    pub fn new(payload: TokenTransactionFromDb) -> Self {
        Self {
            id: Uuid::new_v4(),
            service_id: payload.service_id,
            token_transaction_id: payload.id,
            message_hash: payload.message_hash,
            account_workchain_id: payload.account_workchain_id,
            account_hex: payload.account_hex,
            value: payload.value,
            root_address: payload.root_address,
            transaction_direction: TonTransactionDirection::Send,
            transaction_status: TonTokenTransactionStatus::New,
            event_status: TonEventStatus::New,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct UpdateSendTokenTransactionEvent {
    pub transaction_status: TonTokenTransactionStatus,
}

impl UpdateSendTokenTransactionEvent {
    pub fn new(payload: TokenTransactionFromDb) -> Self {
        Self {
            transaction_status: payload.status,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct CreateReceiveTokenTransactionEvent {
    pub id: Uuid,
    pub service_id: ServiceId,
    pub token_transaction_id: Uuid,
    pub message_hash: String,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub value: BigDecimal,
    pub root_address: String,
    pub transaction_direction: TonTransactionDirection,
    pub transaction_status: TonTokenTransactionStatus,
    pub event_status: TonEventStatus,
}

impl CreateReceiveTokenTransactionEvent {
    pub fn new(payload: TokenTransactionFromDb) -> Self {
        Self {
            id: Uuid::new_v4(),
            service_id: payload.service_id,
            token_transaction_id: payload.id,
            message_hash: payload.message_hash,
            account_workchain_id: payload.account_workchain_id,
            account_hex: payload.account_hex,
            value: payload.value,
            root_address: payload.root_address,
            transaction_direction: TonTransactionDirection::Receive,
            transaction_status: TonTokenTransactionStatus::Done,
            event_status: TonEventStatus::New,
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct TokenTransactionsEventsSearch {
    pub limit: i64,
    pub offset: i64,
    pub created_at_ge: Option<i64>,
    pub created_at_le: Option<i64>,
    pub token_transaction_id: Option<Uuid>,
    pub message_hash: Option<String>,
    pub account_workchain_id: Option<i32>,
    pub account_hex: Option<String>,
    pub root_address: Option<String>,
    pub transaction_direction: Option<TonTransactionDirection>,
    pub transaction_status: Option<TonTokenTransactionStatus>,
    pub event_status: Option<TonEventStatus>,
}
