use dexpa::newtypes::Amount;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use broxus::newtypes::TxHash;

use crate::models::{
    AddressResponse, TonEventStatus, TonTransactionDirection, TonTransactionStatus,
};

#[derive(Debug, Serialize, Deserialize, Clone, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
pub struct AccountTransactionEvent {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub message_hash: TxHash,
    pub account: AddressResponse,
    pub balance_change: Amount,
    pub transaction_direction: TonTransactionDirection,
    pub transaction_status: TonTransactionStatus,
    pub event_status: TonEventStatus,
    pub created_at: i64,
    pub updated_at: i64,
}
