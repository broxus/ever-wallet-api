use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::account_enums::TransactionSendOutputType;
use crate::models::address::Address;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TransactionSend {
    pub id: Uuid,
    pub from_address: Address,
    pub outputs: Vec<TransactionSendOutput>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TransactionSendOutput {
    pub recipient_address: Address,
    pub value: BigDecimal,
    pub output_type: Option<TransactionSendOutputType>,
}
