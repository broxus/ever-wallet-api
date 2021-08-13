use bigdecimal::BigDecimal;
use sentry::types::Uuid;

use crate::models::account_enums::AccountType;
use crate::models::service_id::ServiceId;

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
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct CreateAddressInDb {
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
