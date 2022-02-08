use bigdecimal::{BigDecimal, FromPrimitive};
use nekoton_utils::TrustMe;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::*;
use crate::prelude::*;

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
    pub id: Option<Uuid>,
    pub from_address: Address,
    pub outputs: Vec<PostTonTransactionSendOutputRequest>,
    pub bounce: Option<bool>,
}

impl From<PostTonTransactionSendRequest> for TransactionSend {
    fn from(c: PostTonTransactionSendRequest) -> Self {
        TransactionSend {
            id: c.id.unwrap_or_else(Uuid::new_v4),
            from_address: c.from_address,
            bounce: c.bounce,
            outputs: c.outputs.into_iter().map(From::from).collect(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostTonTransactionConfirmRequest")]
pub struct PostTonTransactionConfirmRequest {
    pub id: Option<Uuid>,
    pub address: Address,
    pub transaction_id: u64,
}

impl From<PostTonTransactionConfirmRequest> for TransactionConfirm {
    fn from(c: PostTonTransactionConfirmRequest) -> Self {
        TransactionConfirm {
            id: c.id.unwrap_or_else(Uuid::new_v4),
            address: c.address,
            transaction_id: c.transaction_id,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostTonTransactionsRequest")]
pub struct PostTonTransactionsRequest {
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

impl From<PostTonTransactionsRequest> for TransactionsSearch {
    fn from(c: PostTonTransactionsRequest) -> Self {
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

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostTonTokenTransactionSendRequest")]
pub struct PostTonTokenTransactionSendRequest {
    pub id: Option<Uuid>,
    pub from_address: Address,
    pub root_address: Address,
    pub recipient_address: Address,
    #[opg("sendGasTo", string, optional)]
    pub send_gas_to: Option<Address>,
    #[opg("value", string)]
    pub value: BigDecimal,
    pub notify_receiver: Option<bool>,
    #[opg("fee", string, optional)]
    pub fee: Option<BigDecimal>,
}

impl From<PostTonTokenTransactionSendRequest> for TokenTransactionSend {
    fn from(c: PostTonTokenTransactionSendRequest) -> Self {
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
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostTonTokenTransactionBurnRequest")]
pub struct PostTonTokenTransactionBurnRequest {
    pub id: Option<Uuid>,
    pub from_address: Address,
    pub root_address: Address,
    #[opg("sendGasTo", string, optional)]
    pub send_gas_to: Option<Address>,
    pub callback_to: Address,
    #[opg("value", string)]
    pub value: BigDecimal,
    #[opg("fee", string, optional)]
    pub fee: Option<BigDecimal>,
}

impl From<PostTonTokenTransactionBurnRequest> for TokenTransactionBurn {
    fn from(c: PostTonTokenTransactionBurnRequest) -> Self {
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

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostTonTokenTransactionMintRequest")]
pub struct PostTonTokenTransactionMintRequest {
    pub id: Option<Uuid>,
    pub from_address: Address,
    pub root_address: Address,
    #[opg("value", string)]
    pub value: BigDecimal,
    pub recipient_address: Address,
    #[opg("deployWalletValue", string, optional)]
    pub deploy_wallet_value: Option<BigDecimal>,
    #[opg("sendGasTo", string, optional)]
    pub send_gas_to: Option<Address>,
    pub notify: Option<bool>,
    #[opg("fee", string, optional)]
    pub fee: Option<BigDecimal>,
}

impl From<PostTonTokenTransactionMintRequest> for TokenTransactionMint {
    fn from(c: PostTonTokenTransactionMintRequest) -> Self {
        TokenTransactionMint {
            id: c.id.unwrap_or_else(Uuid::new_v4),
            from_address: c.from_address,
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
    pub owner_message_hash: Option<String>,
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
            owner_message_hash: c.owner_message_hash,
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

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("MarkAllTransactionEventRequest")]
pub struct MarkAllTransactionEventRequest {
    pub event_status: Option<TonEventStatus>,
}
