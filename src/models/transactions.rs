use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::account_enums::{
    TonTransactionDirection, TonTransactionStatus, TransactionSendOutputType,
};
use crate::models::address::Address;
use crate::models::service_id::ServiceId;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TransactionSend {
    pub id: Uuid,
    pub from_address: Address,
    pub outputs: Vec<TransactionSendOutput>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TransactionSendOutput {
    pub recipient_address: Address,
    pub value: BigDecimal,
    pub output_type: Option<TransactionSendOutputType>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct CreateSendTransaction {
    pub id: Uuid,
    pub service_id: ServiceId,
    pub message_hash: String,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub value: BigDecimal,
    pub direction: TonTransactionDirection,
    pub status: TonTransactionStatus,
    pub aborted: bool,
    pub bounce: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct UpdateSendTransaction {
    pub transaction_hash: Option<String>,
    pub transaction_lt: Option<BigDecimal>,
    pub transaction_timeout: Option<i64>,
    pub transaction_scan_lt: Option<i64>,
    pub sender_workchain_id: Option<i32>,
    pub sender_hex: Option<String>,
    pub messages: Option<serde_json::Value>,
    pub data: Option<serde_json::Value>,
    pub original_value: Option<BigDecimal>,
    pub original_outputs: Option<serde_json::Value>,
    pub value: Option<BigDecimal>,
    pub fee: Option<BigDecimal>,
    pub balance_change: Option<BigDecimal>,
    pub status: TonTransactionStatus,
    pub error: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct CreateReceiveTransaction {
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
}
