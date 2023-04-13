use bigdecimal::BigDecimal;

use crate::models::*;

#[derive(
    Clone,
    Debug,
    Default,
    derive_more::Display,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    derive_more::From,
    derive_more::FromStr,
    derive_more::Into,
    serde::Serialize,
    serde::Deserialize,
    opg::OpgModel,
)]
#[opg(inline, string)]
pub struct Address(pub String);

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct CreateAddress {
    pub account_type: Option<AccountType>,
    pub workchain_id: Option<i32>,
    pub custodians: Option<i32>,
    pub confirmations: Option<i32>,
    pub custodians_public_keys: Option<Vec<String>>,
}

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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct CreateAddressInDb {
    pub id: uuid::Uuid,
    pub service_id: ServiceId,
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
    pub fn new(
        c: CreatedAddress,
        id: uuid::Uuid,
        service_id: ServiceId,
        public_key: String,
        private_key: String,
    ) -> Self {
        Self {
            id,
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
pub struct NetworkAddressData {
    pub workchain_id: i32,
    pub hex: String,
    pub account_status: AccountStatus,
    pub network_balance: BigDecimal,
    pub last_transaction_hash: Option<String>,
    pub last_transaction_lt: Option<String>,
    pub sync_u_time: i64,
}

impl NetworkAddressData {
    pub fn uninit(owner: &ton_block::MsgAddressInt) -> NetworkAddressData {
        NetworkAddressData {
            workchain_id: owner.workchain_id(),
            hex: owner.address().to_hex_string(),
            account_status: AccountStatus::UnInit,
            network_balance: Default::default(),
            last_transaction_hash: None,
            last_transaction_lt: None,
            sync_u_time: 0,
        }
    }
}

#[derive(Debug)]
pub struct AddAddress {
    pub public_key: String,
    pub private_key: String,
    pub address: String,

    pub account_type: Option<AccountType>,
    pub custodians: Option<i32>,
    pub confirmations: Option<i32>,
    pub custodians_public_keys: Option<Vec<String>>,
}
