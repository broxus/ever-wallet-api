use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use ton_abi::Param;
use uuid::Uuid;

use crate::models::*;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TransactionSend {
    pub id: Uuid,
    pub from_address: Address,
    pub outputs: Vec<TransactionSendOutput>,
    pub bounce: Option<bool>,
    pub payload: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct CreateReceiveTransaction {
    pub id: Uuid,
    pub message_hash: String,
    pub transaction_hash: Option<String>,
    pub transaction_lt: Option<BigDecimal>,
    pub transaction_timeout: Option<i64>,
    pub transaction_scan_lt: Option<i64>,
    pub transaction_timestamp: u32,
    pub sender_workchain_id: Option<i32>,
    pub sender_hex: Option<String>,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub messages: Option<serde_json::Value>,
    pub messages_hash: Option<serde_json::Value>,
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
    pub multisig_transaction_id: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TransactionConfirm {
    pub id: Uuid,
    pub address: Address,
    pub transaction_id: u64,
}

/*#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenTransactionSend {
    pub id: uuid::Uuid,
    pub owner: MsgAddressInt,
    pub token_wallet: MsgAddressInt,
    pub version: TokenWalletVersion,
    pub destination: TransferRecipient,
    pub send_gas_to: MsgAddressInt,
    pub tokens: BigDecimal,
    pub notify_receiver: bool,
    pub attached_amount: u64,
}*/

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
    pub original_value: Option<BigDecimal>,
    pub original_outputs: Option<serde_json::Value>,
    pub direction: TonTransactionDirection,
    pub status: TonTransactionStatus,
    pub aborted: bool,
    pub bounce: bool,
}

impl CreateSendTransaction {
    pub fn new(s: SentTransaction, service_id: ServiceId) -> Self {
        Self {
            id: s.id,
            service_id,
            message_hash: s.message_hash,
            account_workchain_id: s.account_workchain_id,
            account_hex: s.account_hex,
            original_value: s.original_value,
            original_outputs: s.original_outputs,
            direction: TonTransactionDirection::Send,
            status: TonTransactionStatus::New,
            aborted: s.aborted,
            bounce: s.bounce,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct UpdateSendTransaction {
    pub transaction_hash: Option<String>,
    pub transaction_lt: Option<BigDecimal>,
    pub transaction_scan_lt: Option<i64>,
    pub transaction_timestamp: Option<u32>,
    pub sender_workchain_id: Option<i32>,
    pub sender_hex: Option<String>,
    pub messages: Option<serde_json::Value>,
    pub messages_hash: Option<serde_json::Value>,
    pub data: Option<serde_json::Value>,
    pub value: Option<BigDecimal>,
    pub fee: Option<BigDecimal>,
    pub balance_change: Option<BigDecimal>,
    pub status: TonTransactionStatus,
    pub error: Option<String>,
    pub multisig_transaction_id: Option<i64>,
}

impl UpdateSendTransaction {
    pub fn error(error: String) -> Self {
        Self {
            transaction_hash: None,
            transaction_lt: None,
            transaction_scan_lt: None,
            transaction_timestamp: None,
            sender_workchain_id: None,
            sender_hex: None,
            messages_hash: None,
            messages: None,
            data: None,
            value: None,
            fee: None,
            balance_change: None,
            status: TonTransactionStatus::Error,
            error: Some(error),
            multisig_transaction_id: None,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct UpdateSentTransaction {
    pub message_hash: String,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub input: UpdateSendTransaction,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct TransactionsSearch {
    pub id: Option<Uuid>,
    pub message_hash: Option<String>,
    pub transaction_hash: Option<String>,
    pub account: Option<String>,
    pub status: Option<TonTransactionStatus>,
    pub direction: Option<TonTransactionDirection>,
    pub created_at_min: Option<i64>,
    pub created_at_max: Option<i64>,
    pub ordering: Option<TransactionsSearchOrdering>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct SentTransaction {
    pub id: Uuid,
    pub message_hash: String,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub original_value: Option<BigDecimal>,
    pub original_outputs: Option<serde_json::Value>,
    pub aborted: bool,
    pub bounce: bool,
}

#[derive(Clone, Deserialize, Debug)]
pub struct InputParam {
    pub param: Param,
    pub value: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FunctionDetails {
    pub function_name: String,
    pub input_params: Vec<InputParam>,
    pub output_params: Vec<Param>,
    pub headers: Vec<Param>,
}
