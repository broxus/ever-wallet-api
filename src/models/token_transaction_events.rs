use bigdecimal::BigDecimal;
use uuid::Uuid;

use crate::models::account_enums::{
    TonEventStatus, TonTokenTransactionStatus, TonTransactionDirection,
};
use crate::models::service_id::ServiceId;

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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct UpdateSendTokenTransactionEvent {
    pub transaction_status: TonTokenTransactionStatus,
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
