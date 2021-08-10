use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::account_enums::{AccountType, TonEventStatus, AccountAddressType, PostTonTransactionSendOutputType};
use crate::models::address::Address;

#[derive(Debug, Deserialize, Serialize, Clone, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
pub struct AccountAddressRequest {
    pub address_type: AccountAddressType,
}

#[derive(Debug, Deserialize, Serialize, Clone, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
pub struct CreateAddressRequest {
    pub account_type: Option<AccountType>,
    pub workchain_id: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize, Clone, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
pub struct BalanceRequest {
    pub api_key: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
pub struct PostTransactionSendRequest {
    pub source_address: Address,
    pub outputs: Vec<PostTonTransactionSendOutput>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PostTonTransactionSendOutput {
    pub id: Uuid,
    pub recipient_address: Address,
    pub value: BigDecimal,
    pub output_type: Option<PostTonTransactionSendOutputType>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PostTonTransactionSendRequest {
    pub id: Uuid,
    pub from_address: Address,
    pub outputs: Vec<PostTonTransactionSendOutputRequest>,
}

#[derive(Debug, Deserialize, Serialize, Clone, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
pub struct PostTonMarkEventsRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PostTonTransactionEventsRequest {
    pub event_status: TonEventStatus,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PostTonTransactionSendOutputRequest {
    pub recipient_address: Address,
    pub value: BigDecimal,
    pub output_type: Option<PostTonTransactionSendOutputType>,
}

#[derive(Debug, Deserialize, Serialize, Clone, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
pub struct PostAddressBalanceRequest {
    pub address: Address,
}

#[derive(Debug, Deserialize, Serialize, Clone, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
pub struct PostTransactionHistoryRequest {
    pub address: Address,
    pub offset: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
pub struct PostSetOffsetRequest {
    pub address: Address,
    pub offset: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
pub struct PostGetOffsetRequest {
    pub address: Address,
}

#[derive(Debug, Deserialize, Serialize, Clone, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
pub struct TonTransactionStatusRequest {
    pub transaction_id: Uuid,
}
