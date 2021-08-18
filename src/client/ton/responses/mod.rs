use bigdecimal::BigDecimal;
use uuid::Uuid;

use crate::models::account_enums::{
    AccountType, TonTokenTransactionStatus, TonTransactionDirection, TonTransactionStatus,
};
use crate::models::address::CreateAddressInDb;
use crate::models::service_id::ServiceId;
use crate::models::token_transactions::CreateSendTokenTransaction;
use crate::models::transactions::CreateSendTransaction;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct CreatedAddress {
    pub workchain_id: i32,
    pub hex: String,
    pub base64url: String,
    pub public_key: String,
    pub private_key: String,
    pub account_type: AccountType,
    pub custodians: Option<i32>,
    pub confirmations: Option<i32>,
    pub custodians_public_keys: Option<serde_json::Value>,
}

impl CreateAddressInDb {
    pub fn new(c: CreatedAddress, service_id: ServiceId) -> Self {
        Self {
            service_id,
            workchain_id: c.workchain_id,
            hex: c.hex,
            base64url: c.base64url,
            public_key: c.public_key,
            private_key: c.private_key,
            account_type: c.account_type,
            custodians: c.custodians,
            confirmations: c.confirmations,
            custodians_public_keys: c.custodians_public_keys,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct SentTransaction {
    pub id: Uuid,
    pub message_hash: String,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub value: BigDecimal,
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
            value: -s.value,
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
