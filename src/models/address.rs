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
