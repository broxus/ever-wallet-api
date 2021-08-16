use bigdecimal::BigDecimal;

use crate::models::service_id::ServiceId;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct CreateTokenBalanceInDb {
    pub service_id: ServiceId,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub balance: BigDecimal,
    pub root_address: String,
}
