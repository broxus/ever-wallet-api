use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::*;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct CreateTokenTransaction {
    pub id: Uuid,
    pub transaction_hash: Option<String>,
    pub transaction_timestamp: u32,
    pub message_hash: String,
    pub owner_message_hash: Option<String>,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub sender_workchain_id: Option<i32>,
    pub sender_hex: Option<String>,
    pub value: BigDecimal,
    pub root_address: String,
    pub payload: Option<Vec<u8>>,
    pub error: Option<String>,
    pub block_hash: String,
    pub block_time: i32,
    pub direction: TonTransactionDirection,
    pub status: TonTokenTransactionStatus,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenTransactionSend {
    pub id: Uuid,
    pub from_address: Address,
    pub root_address: Address,
    pub recipient_address: Address,
    pub value: BigDecimal,
    pub notify_receiver: bool,
    pub send_gas_to: Option<Address>,
    pub fee: BigDecimal,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenTransactionBurn {
    pub id: Uuid,
    pub from_address: Address,
    pub root_address: Address,
    pub value: BigDecimal,
    pub send_gas_to: Option<Address>,
    pub callback_to: Address,
    pub fee: BigDecimal,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenTransactionMint {
    pub id: Uuid,
    pub from_address: Address,
    pub root_address: Address,
    pub value: BigDecimal,
    pub recipient_address: Address,
    pub deploy_wallet_value: BigDecimal,
    pub send_gas_to: Option<Address>,
    pub notify: bool,
    pub fee: BigDecimal,
}
