use derive_more::Constructor;
use opg::OpgModel;
use serde::Deserialize;
use uuid::Uuid;

use crate::models::*;
use crate::prelude::*;

#[derive(Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TonTransactionEventsRequest")]
pub struct TonTransactionEventsRequest {
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

impl From<TonTransactionEventsRequest> for TransactionsEventsSearch {
    fn from(c: TonTransactionEventsRequest) -> Self {
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

#[derive(Deserialize, OpgModel, Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("TonMarkEventsRequest")]
pub struct TonMarkEventsRequest {
    pub id: Uuid,
}

#[derive(Deserialize, OpgModel, Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("MarkAllTransactionEventRequest")]
pub struct MarkAllTransactionEventRequest {
    pub event_status: Option<TonEventStatus>,
}

#[derive(Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TonTokenTransactionEventsRequest")]
pub struct TonTokenTransactionEventsRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub created_at_ge: Option<i64>,
    pub created_at_le: Option<i64>,
    pub token_transaction_id: Option<Uuid>,
    pub message_hash: Option<String>,
    pub account_workchain_id: Option<i32>,
    pub account_hex: Option<String>,
    pub owner_message_hash: Option<String>,
    pub root_address: Option<String>,
    pub transaction_direction: Option<TonTransactionDirection>,
    pub transaction_status: Option<TonTokenTransactionStatus>,
    pub event_status: Option<TonEventStatus>,
}

impl From<TonTokenTransactionEventsRequest> for TokenTransactionsEventsSearch {
    fn from(c: TonTokenTransactionEventsRequest) -> Self {
        TokenTransactionsEventsSearch {
            limit: c.limit.unwrap_or(MAX_LIMIT_SEARCH),
            offset: c.offset.unwrap_or(0),
            created_at_ge: c.created_at_ge,
            created_at_le: c.created_at_le,
            token_transaction_id: c.token_transaction_id,
            message_hash: c.message_hash,
            account_workchain_id: c.account_workchain_id,
            account_hex: c.account_hex,
            owner_message_hash: c.owner_message_hash,
            root_address: c.root_address,
            transaction_direction: c.transaction_direction,
            transaction_status: c.transaction_status,
            event_status: c.event_status,
        }
    }
}

#[derive(Deserialize, OpgModel, Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("TonTokenMarkEventsRequest")]
pub struct TonTokenMarkEventsRequest {
    pub id: Uuid,
}
