use std::str::FromStr;

use bigdecimal::BigDecimal;
use nekoton_utils::pack_std_smc_addr;
use opg::OpgModel;
use serde::{Deserialize, Serialize};
use ton_block::MsgAddressInt;
use uuid::Uuid;

use crate::api::*;
use crate::models::*;

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TransactionResponse")]
pub struct TransactionResponse {
    pub status: TonStatus,
    pub data: Option<TransactionDataResponse>,
    pub error_message: Option<String>,
}

impl From<Result<TransactionDataResponse, Error>> for TransactionResponse {
    fn from(r: Result<TransactionDataResponse, Error>) -> Self {
        match r {
            Ok(data) => Self {
                status: TonStatus::Ok,
                error_message: None,
                data: Some(data),
            },
            Err(e) => Self {
                status: TonStatus::Error,
                error_message: Some(e.to_string()),
                data: None,
            },
        }
    }
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TransactionDataResponse")]
pub struct TransactionDataResponse {
    #[opg("id", string)]
    pub id: Uuid,
    pub message_hash: String,
    pub transaction_hash: Option<String>,
    pub transaction_lt: Option<String>,
    pub transaction_timeout: Option<i64>,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub transaction_timestamp: Option<i64>,
    pub account: Account,
    pub sender: Option<Account>,
    #[opg("value", string)]
    pub value: Option<BigDecimal>,
    #[opg("originalValue", string)]
    pub original_value: Option<BigDecimal>,
    #[opg("fee", string)]
    pub fee: Option<BigDecimal>,
    #[opg("balanceСhange", string)]
    pub balance_change: BigDecimal,
    pub out_messages: Option<Vec<TransactionMessage>>,
    pub original_outputs: Option<Vec<TransactionOutput>>,
    pub direction: TonTransactionDirection,
    pub status: TonTransactionStatus,
    pub aborted: bool,
    pub bounce: bool,
    pub error: Option<String>,
    pub multisig_transaction_id: Option<i64>,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub created_at: i64,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub updated_at: i64,
}

impl From<TransactionDb> for TransactionDataResponse {
    fn from(c: TransactionDb) -> Self {
        let sender = if let (Some(sender_hex), Some(sender_workchain_id)) =
            (c.sender_hex, c.sender_workchain_id)
        {
            let sender =
                MsgAddressInt::from_str(&format!("{}:{}", sender_workchain_id, sender_hex))
                    .unwrap_or_default();
            let sender_base64url = Address(pack_std_smc_addr(true, &sender, true).unwrap());
            Some(Account {
                workchain_id: sender_workchain_id,
                hex: Address(sender_hex),
                base64url: sender_base64url,
            })
        } else {
            None
        };

        let original_outputs = if let Some(outputs) = c.original_outputs {
            serde_json::from_value(outputs.clone())
                .map(|original_outputs: Vec<TransactionSendOutput>| {
                    original_outputs
                        .into_iter()
                        .map(|output| {
                            let output_address =
                                nekoton_utils::repack_address(&output.recipient_address.0)
                                    .unwrap_or_default();
                            let output_base64url =
                                Address(pack_std_smc_addr(true, &output_address, true).unwrap());
                            TransactionOutput {
                                value: output.value,
                                recipient: Account {
                                    workchain_id: output_address.workchain_id(),
                                    hex: Address(output_address.address().to_hex_string()),
                                    base64url: output_base64url,
                                },
                            }
                        })
                        .collect()
                })
                .or_else(|_| serde_json::from_value(outputs))
                .ok()
        } else {
            None
        };

        let account =
            MsgAddressInt::from_str(&format!("{}:{}", c.account_workchain_id, c.account_hex))
                .unwrap();
        let base64url = Address(pack_std_smc_addr(true, &account, true).unwrap());

        TransactionDataResponse {
            id: c.id,
            message_hash: c.message_hash,
            transaction_hash: c.transaction_hash,
            transaction_lt: c.transaction_lt.map(|v| v.to_string()),
            transaction_timeout: c.transaction_timeout,
            account: Account {
                workchain_id: c.account_workchain_id,
                hex: Address(c.account_hex),
                base64url,
            },
            sender,
            value: c.value,
            original_value: c.original_value,
            fee: c.fee,
            balance_change: c.balance_change.unwrap_or_default(),
            out_messages: c.messages.and_then(|m| serde_json::from_value(m).ok()),
            original_outputs,
            direction: c.direction,
            status: c.status,
            aborted: c.aborted,
            bounce: c.bounce,
            transaction_timestamp: c.transaction_timestamp.map(|t| t.timestamp_millis()),
            created_at: c.created_at.timestamp_millis(),
            updated_at: c.updated_at.timestamp_millis(),
            error: c.error,
            multisig_transaction_id: c.multisig_transaction_id,
        }
    }
}

#[derive(Serialize, Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TransactionMessage")]
pub struct TransactionMessage {
    pub message_hash: String,
    #[opg("value", string)]
    pub value: BigDecimal,
    #[opg("fee", string)]
    pub fee: BigDecimal,
    pub recipient: Account,
}

#[derive(Serialize, Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TransactionOutput")]
pub struct TransactionOutput {
    #[opg("value", string)]
    pub value: BigDecimal,
    pub recipient: Account,
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TonTransactionsResponse")]
pub struct TonTransactionsResponse {
    pub status: TonStatus,
    pub data: Option<TransactionsResponse>,
    pub error_message: Option<String>,
}

impl From<Result<TransactionsResponse, Error>> for TonTransactionsResponse {
    fn from(r: Result<TransactionsResponse, Error>) -> Self {
        match r {
            Ok(data) => Self {
                status: TonStatus::Ok,
                error_message: None,
                data: Some(data),
            },
            Err(e) => Self {
                status: TonStatus::Error,
                error_message: Some(e.to_string()),
                data: None,
            },
        }
    }
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TransactionsResponse")]
pub struct TransactionsResponse {
    pub count: i32,
    pub items: Vec<TransactionDataResponse>,
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TokenTransactionResponse")]
pub struct TokenTransactionResponse {
    pub status: TonStatus,
    pub data: Option<TokenTransactionDataResponse>,
    pub error_message: Option<String>,
}

impl From<Result<TokenTransactionDataResponse, Error>> for TokenTransactionResponse {
    fn from(r: Result<TokenTransactionDataResponse, Error>) -> Self {
        match r {
            Ok(data) => Self {
                status: TonStatus::Ok,
                error_message: None,
                data: Some(data),
            },
            Err(e) => Self {
                status: TonStatus::Error,
                error_message: Some(e.to_string()),
                data: None,
            },
        }
    }
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TokenTransactionDataResponse")]
pub struct TokenTransactionDataResponse {
    pub id: Uuid,
    pub transaction_hash: Option<String>,
    pub message_hash: String,
    pub account: Account,
    #[opg("value", string)]
    pub value: BigDecimal,
    pub root_address: String,
    pub error: Option<String>,
    pub block_hash: Option<String>,
    pub block_time: Option<i32>,
    pub direction: TonTransactionDirection,
    pub status: TonTokenTransactionStatus,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub created_at: i64,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub updated_at: i64,
}

impl From<TokenTransactionFromDb> for TokenTransactionDataResponse {
    fn from(c: TokenTransactionFromDb) -> Self {
        let account =
            MsgAddressInt::from_str(&format!("{}:{}", c.account_workchain_id, c.account_hex))
                .unwrap();
        let base64url = Address(pack_std_smc_addr(true, &account, true).unwrap());

        TokenTransactionDataResponse {
            id: c.id,
            message_hash: c.message_hash,
            transaction_hash: c.transaction_hash,
            account: Account {
                workchain_id: c.account_workchain_id,
                hex: Address(c.account_hex),
                base64url,
            },
            value: c.value,
            root_address: c.root_address,
            error: c.error,
            block_hash: c.block_hash,
            block_time: c.block_time,
            direction: c.direction,
            status: c.status,
            created_at: c.created_at.timestamp_millis(),
            updated_at: c.updated_at.timestamp_millis(),
        }
    }
}
