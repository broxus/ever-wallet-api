use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::account_enums::{
    AccountAddressType, AccountType, TonEventStatus, TransactionSendOutputType,
};
use crate::models::address::{Address, CreateAddress};
use crate::models::token_transactions::TokenTransactionSend;
use crate::models::transactions::{TransactionSend, TransactionSendOutput};

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("CreateAddressRequest")]
pub struct CreateAddressRequest {
    pub account_type: Option<AccountType>,
    pub workchain_id: Option<i32>,
    pub custodians: Option<i32>,
    pub confirmations: Option<i32>,
    #[opg("custodiansPublicKeys", any)]
    pub custodians_public_keys: Option<serde_json::Value>,
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
}

impl From<PostTonTransactionSendRequest> for TransactionSend {
    fn from(c: PostTonTransactionSendRequest) -> Self {
        TransactionSend {
            id: c.id,
            from_address: c.from_address,
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
}

impl From<PostTonTokenTransactionSendRequest> for TokenTransactionSend {
    fn from(c: PostTonTokenTransactionSendRequest) -> Self {
        TokenTransactionSend {
            id: c.id,
            from_address: c.from_address,
            root_address: c.root_address,
            recipient_address: c.recipient_address,
            value: c.value,
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
    pub event_status: TonEventStatus,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostTonTokenTransactionEventsRequest")]
pub struct PostTonTokenTransactionEventsRequest {
    pub event_status: TonEventStatus,
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
