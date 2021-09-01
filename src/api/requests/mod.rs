use std::str::FromStr;

use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::account_enums::{
    AccountAddressType, AccountType, TonEventStatus, TonTokenTransactionStatus,
    TonTransactionDirection, TonTransactionStatus, TransactionSendOutputType,
};
use crate::models::address::{Address, CreateAddress};
use crate::models::token_transaction_events::TokenTransactionsEventsSearch;
use crate::models::token_transactions::TokenTransactionSend;
use crate::models::transaction_events::TransactionsEventsSearch;
use crate::models::transactions::{TransactionSend, TransactionSendOutput};
use crate::prelude::{MAX_LIMIT_SEARCH, TOKEN_FEE};

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("CreateAddressRequest")]
pub struct CreateAddressRequest {
    pub account_type: Option<AccountType>,
    pub workchain_id: Option<i32>,
    pub custodians: Option<i32>,
    pub confirmations: Option<i32>,
    pub custodians_public_keys: Option<Vec<String>>,
}

impl From<CreateAddressRequest> for CreateAddress {
    fn from(c: CreateAddressRequest) -> Self {
        CreateAddress {
            account_type: c.account_type,
            workchain_id: c.workchain_id,
            custodians: c.custodians,
            confirmations: c.confirmations,
            custodians_public_keys: c.custodians_public_keys,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("BalanceRequest")]
pub struct BalanceRequest {
    pub api_key: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostTonTransactionSendRequest")]
pub struct PostTonTransactionSendRequest {
    pub id: Uuid,
    pub from_address: Address,
    pub outputs: Vec<PostTonTransactionSendOutputRequest>,
    pub bounce: Option<bool>,
}

impl From<PostTonTransactionSendRequest> for TransactionSend {
    fn from(c: PostTonTransactionSendRequest) -> Self {
        TransactionSend {
            id: c.id,
            from_address: c.from_address,
            bounce: c.bounce,
            outputs: c.outputs.into_iter().map(From::from).collect(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostTonTokenTransactionSendRequest")]
pub struct PostTonTokenTransactionSendRequest {
    pub id: Uuid,
    pub from_address: Address,
    pub root_address: String,
    pub recipient_address: Address,
    #[opg("value", string)]
    pub value: BigDecimal,
    pub notify_receiver: Option<bool>,
    pub bounce: Option<bool>,
    #[opg("fee", string, optional)]
    pub fee: Option<BigDecimal>,
}

impl From<PostTonTokenTransactionSendRequest> for TokenTransactionSend {
    fn from(c: PostTonTokenTransactionSendRequest) -> Self {
        TokenTransactionSend {
            id: c.id,
            from_address: c.from_address,
            root_address: c.root_address,
            recipient_address: c.recipient_address,
            value: c.value,
            bounce: c.bounce,
            notify_receiver: c.notify_receiver.unwrap_or(false),
            fee: c.fee.unwrap_or(BigDecimal::from_str(TOKEN_FEE).unwrap()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("PostTonMarkEventsRequest")]
pub struct PostTonMarkEventsRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("PostTonTokenMarkEventsRequest")]
pub struct PostTonTokenMarkEventsRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostTonTransactionEventsRequest")]
pub struct PostTonTransactionEventsRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub created_at_ge: Option<i64>,
    pub created_at_le: Option<i64>,
    pub transaction_id: Option<Uuid>,
    pub message_hash: Option<String>,
    pub account_workchain_id: Option<i32>,
    pub account_hex: Option<String>,
    pub transaction_direction: Option<TonTransactionDirection>,
    pub transaction_status: Option<TonTransactionStatus>,
    pub event_status: Option<TonEventStatus>,
}

impl From<PostTonTransactionEventsRequest> for TransactionsEventsSearch {
    fn from(c: PostTonTransactionEventsRequest) -> Self {
        TransactionsEventsSearch {
            limit: c.limit.unwrap_or(MAX_LIMIT_SEARCH),
            offset: c.offset.unwrap_or(0),
            created_at_ge: c.created_at_ge,
            created_at_le: c.created_at_le,
            transaction_id: c.transaction_id,
            message_hash: c.message_hash,
            account_workchain_id: c.account_workchain_id,
            account_hex: c.account_hex,
            transaction_direction: c.transaction_direction,
            transaction_status: c.transaction_status,
            event_status: c.event_status,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostTonTokenTransactionEventsRequest")]
pub struct PostTonTokenTransactionEventsRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
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

impl From<PostTonTokenTransactionEventsRequest> for TokenTransactionsEventsSearch {
    fn from(c: PostTonTokenTransactionEventsRequest) -> Self {
        TokenTransactionsEventsSearch {
            limit: c.limit.unwrap_or(MAX_LIMIT_SEARCH),
            offset: c.offset.unwrap_or(0),
            created_at_ge: c.created_at_ge,
            created_at_le: c.created_at_le,
            token_transaction_id: c.token_transaction_id,
            message_hash: c.message_hash,
            account_workchain_id: c.account_workchain_id,
            account_hex: c.account_hex,
            root_address: c.root_address,
            transaction_direction: c.transaction_direction,
            transaction_status: c.transaction_status,
            event_status: c.event_status,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostTonTransactionSendOutputRequest")]
pub struct PostTonTransactionSendOutputRequest {
    pub recipient_address: Address,
    #[opg("value", string)]
    pub value: BigDecimal,
    pub output_type: Option<TransactionSendOutputType>,
}

impl From<PostTonTransactionSendOutputRequest> for TransactionSendOutput {
    fn from(c: PostTonTransactionSendOutputRequest) -> Self {
        TransactionSendOutput {
            recipient_address: c.recipient_address,
            value: c.value,
            output_type: c.output_type,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("PostAddressBalanceRequest")]
pub struct PostAddressBalanceRequest {
    pub address: Address,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("PostTransactionHistoryRequest")]
pub struct PostTransactionHistoryRequest {
    pub address: Address,
    pub offset: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("PostSetOffsetRequest")]
pub struct PostSetOffsetRequest {
    pub address: Address,
    pub offset: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("PostGetOffsetRequest")]
pub struct PostGetOffsetRequest {
    pub address: Address,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("TonTransactionStatusRequest")]
pub struct TonTransactionStatusRequest {
    #[opg("transactionId", string)]
    pub transaction_id: Uuid,
}
