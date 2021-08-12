use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::account_enums::{
    AccountAddressType, AccountType, TonEventStatus, TransactionSendOutputType,
};
use crate::models::address::{Address, CreateAddress};
use crate::models::transactions::{TransactionSend, TransactionSendOutput};

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("AccountAddressRequest")]
pub struct AccountAddressRequest {
    pub address_type: AccountAddressType,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("CreateAddressRequest")]
pub struct CreateAddressRequest {
    pub account_type: Option<AccountType>,
    pub workchain_id: Option<i32>,
}

impl From<CreateAddressRequest> for CreateAddress {
    fn from(c: CreateAddressRequest) -> Self {
        CreateAddress {
            account_type: c.account_type,
            workchain_id: c.workchain_id,
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
    #[opg("id", string)]
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

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("PostTonMarkEventsRequest")]
pub struct PostTonMarkEventsRequest {
    #[opg("id", string)]
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
