use bigdecimal::BigDecimal;
use uuid::Uuid;

use crate::models::*;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct CreatedAddress {
    pub workchain_id: i32,
    pub hex: String,
    pub base64url: String,
    pub public_key: Vec<u8>,
    pub private_key: Vec<u8>,
    pub account_type: AccountType,
    pub custodians: Option<i32>,
    pub confirmations: Option<i32>,
    pub custodians_public_keys: Option<Vec<String>>,
}

impl CreateAddressInDb {
    pub fn new(
        c: CreatedAddress,
        service_id: ServiceId,
        public_key: String,
        private_key: String,
    ) -> Self {
        Self {
            service_id,
            workchain_id: c.workchain_id,
            hex: c.hex,
            base64url: c.base64url,
            public_key,
            private_key,
            account_type: c.account_type,
            custodians: c.custodians,
            confirmations: c.confirmations,
            custodians_public_keys: c
                .custodians_public_keys
                .map(|c| serde_json::to_value(c).unwrap_or_default()),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct SentTransaction {
    pub id: Uuid,
    pub message_hash: String,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub original_value: Option<BigDecimal>,
    pub original_outputs: Option<serde_json::Value>,
    pub aborted: bool,
    pub bounce: bool,
}

impl CreateSendTransaction {
    pub fn new(s: SentTransaction, service_id: ServiceId) -> Self {
        Self {
            id: s.id,
            service_id,
            message_hash: s.message_hash,
            account_workchain_id: s.account_workchain_id,
            account_hex: s.account_hex,
            original_value: s.original_value,
            original_outputs: s.original_outputs,
            direction: TonTransactionDirection::Send,
            status: TonTransactionStatus::New,
            aborted: s.aborted,
            bounce: s.bounce,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct SentTokenTransaction {
    pub id: Uuid,
    pub message_hash: String,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub value: BigDecimal,
    pub root_address: String,
    pub notify_receiver: bool,
    pub fee: BigDecimal,
}

impl CreateSendTokenTransaction {
    pub fn new(s: SentTokenTransaction, service_id: ServiceId) -> Self {
        Self {
            id: s.id,
            service_id,
            message_hash: s.message_hash,
            account_workchain_id: s.account_workchain_id,
            account_hex: s.account_hex,
            value: -s.value,
            root_address: s.root_address,
            direction: TonTransactionDirection::Send,
            status: TonTokenTransactionStatus::New,
        }
    }
}
