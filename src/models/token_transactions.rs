use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::account_enums::{TonTokenTransactionStatus, TonTransactionDirection};
use crate::models::address::Address;
use crate::models::service_id::ServiceId;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenTransactionSend {
    pub id: Uuid,
    pub from_address: Address,
    pub root_address: String,
    pub recipient_address: Address,
    pub value: BigDecimal,
    pub notify_receiver: bool,
    pub fee: BigDecimal,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct CreateSendTokenTransaction {
    pub id: Uuid,
    pub service_id: ServiceId,
    pub message_hash: String,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub value: BigDecimal,
    pub root_address: String,
    pub direction: TonTransactionDirection,
    pub status: TonTokenTransactionStatus,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct UpdateSendTokenTransaction {
    pub transaction_hash: Option<String>,
    pub payload: Option<Vec<u8>>,
    pub block_hash: Option<String>,
    pub block_time: Option<i32>,
    pub status: TonTokenTransactionStatus,
    pub error: Option<String>,
}
impl UpdateSendTokenTransaction {
    pub fn error(error: String) -> Self {
        Self {
            transaction_hash: None,
            payload: None,
            block_hash: None,
            block_time: None,
            status: TonTokenTransactionStatus::Error,
            error: Some(error),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct CreateReceiveTokenTransaction {
    pub id: Uuid,
    pub transaction_hash: Option<String>,
    pub message_hash: String,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub value: BigDecimal,
    pub root_address: String,
    pub payload: Option<Vec<u8>>,
    pub error: Option<String>,
    pub block_hash: String,
    pub block_time: i32,
    pub direction: TonTransactionDirection,
    pub status: TonTokenTransactionStatus,
}
