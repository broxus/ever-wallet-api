use crate::models::account_enums::AccountType;

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
