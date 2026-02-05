use bigdecimal::BigDecimal;
use derive_more::Constructor;
use num_traits::FromPrimitive;

use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::models::*;
use crate::prelude::*;

#[derive(Deserialize, Constructor, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TonTransactionSendRequest {
    pub id: Option<Uuid>,
    pub from_address: Address,
    pub outputs: Vec<TonTransactionSendOutputRequest>,
    pub bounce: Option<bool>,
    pub payload: Option<String>,
}

impl From<TonTransactionSendRequest> for TransactionSend {
    fn from(c: TonTransactionSendRequest) -> Self {
        TransactionSend {
            id: c.id.unwrap_or_else(Uuid::new_v4),
            from_address: c.from_address,
            bounce: c.bounce,
            outputs: c.outputs.into_iter().map(From::from).collect(),
            payload: c.payload,
        }
    }
}

#[derive(Deserialize, Constructor, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TonTransactionSendOutputRequest {
    pub recipient_address: Address,
    pub value: BigDecimal,
    pub output_type: Option<TransactionSendOutputType>,
}

impl From<TonTransactionSendOutputRequest> for TransactionSendOutput {
    fn from(c: TonTransactionSendOutputRequest) -> Self {
        TransactionSendOutput {
            recipient_address: c.recipient_address,
            value: c.value,
            output_type: c.output_type,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TonTransactionConfirmRequest {
    pub id: Option<Uuid>,
    pub address: Address,
    pub transaction_id: u64,
}

impl From<TonTransactionConfirmRequest> for TransactionConfirm {
    fn from(c: TonTransactionConfirmRequest) -> Self {
        TransactionConfirm {
            id: c.id.unwrap_or_else(Uuid::new_v4),
            address: c.address,
            transaction_id: c.transaction_id,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TonTransactionsRequest {
    pub id: Option<Uuid>,
    pub message_hash: Option<String>,
    pub transaction_hash: Option<String>,
    pub account: Option<String>,
    pub status: Option<TonTransactionStatus>,
    pub direction: Option<TonTransactionDirection>,
    pub created_at_min: Option<i64>,
    pub created_at_max: Option<i64>,
    pub ordering: Option<TransactionsSearchOrdering>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<TonTransactionsRequest> for TransactionsSearch {
    fn from(c: TonTransactionsRequest) -> Self {
        TransactionsSearch {
            limit: c.limit.unwrap_or(MAX_LIMIT_SEARCH),
            offset: c.offset.unwrap_or(0),
            id: c.id,
            message_hash: c.message_hash,
            transaction_hash: c.transaction_hash,
            account: c.account,
            status: c.status,
            direction: c.direction,
            created_at_min: c.created_at_min,
            created_at_max: c.created_at_max,
            ordering: c.ordering,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TonTokenTransactionSendRequest {
    pub id: Option<Uuid>,
    pub from_address: Address,
    pub root_address: Address,
    pub recipient_address: Address,
    pub send_gas_to: Option<Address>,
    pub value: BigDecimal,
    pub notify_receiver: Option<bool>,
    pub fee: Option<BigDecimal>,
    pub payload: Option<String>,
}

impl From<TonTokenTransactionSendRequest> for TokenTransactionSend {
    fn from(c: TonTokenTransactionSendRequest) -> Self {
        TokenTransactionSend {
            id: c.id.unwrap_or_else(Uuid::new_v4),
            from_address: c.from_address,
            root_address: c.root_address,
            recipient_address: c.recipient_address,
            send_gas_to: c.send_gas_to,
            value: c.value,
            notify_receiver: c.notify_receiver.unwrap_or(false),
            fee: c
                .fee
                .unwrap_or_else(|| BigDecimal::from_u64(TOKEN_FEE).trust_me()),
            payload: c.payload,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TonTokenTransactionBurnRequest {
    pub id: Option<Uuid>,
    pub from_address: Address,
    pub root_address: Address,
    pub send_gas_to: Option<Address>,
    pub callback_to: Address,
    pub value: BigDecimal,
    pub fee: Option<BigDecimal>,
}

impl From<TonTokenTransactionBurnRequest> for TokenTransactionBurn {
    fn from(c: TonTokenTransactionBurnRequest) -> Self {
        TokenTransactionBurn {
            id: c.id.unwrap_or_else(Uuid::new_v4),
            from_address: c.from_address,
            root_address: c.root_address,
            send_gas_to: c.send_gas_to,
            callback_to: c.callback_to,
            value: c.value,
            fee: c
                .fee
                .unwrap_or_else(|| BigDecimal::from_u64(TOKEN_FEE).trust_me()),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TonTokenTransactionMintRequest {
    pub id: Option<Uuid>,
    pub owner_address: Address,
    pub root_address: Address,
    pub value: BigDecimal,
    pub recipient_address: Address,
    pub deploy_wallet_value: Option<BigDecimal>,
    pub send_gas_to: Option<Address>,
    pub notify: Option<bool>,
    pub fee: Option<BigDecimal>,
}

impl From<TonTokenTransactionMintRequest> for TokenTransactionMint {
    fn from(c: TonTokenTransactionMintRequest) -> Self {
        TokenTransactionMint {
            id: c.id.unwrap_or_else(Uuid::new_v4),
            owner_address: c.owner_address,
            root_address: c.root_address,
            value: c.value,
            recipient_address: c.recipient_address,
            send_gas_to: c.send_gas_to,
            notify: c.notify.unwrap_or(false),
            deploy_wallet_value: c
                .deploy_wallet_value
                .unwrap_or_else(|| BigDecimal::from_u64(DEPLOY_TOKEN_VALUE).trust_me()),
            fee: c
                .fee
                .unwrap_or_else(|| BigDecimal::from_u64(TOKEN_FEE).trust_me()),
        }
    }
}
