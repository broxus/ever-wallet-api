use bigdecimal::BigDecimal;

use crate::models::account_enums::AccountStatus;
use crate::models::service_id::ServiceId;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct CreateTokenBalanceInDb {
    pub service_id: ServiceId,
    pub account_workchain_id: i32,
    pub account_hex: String,
    pub balance: BigDecimal,
    pub root_address: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq)]
pub struct NetworkTokenAddressData {
    pub workchain_id: i32,
    pub hex: String,
    pub root_address: String,
    pub network_balance: BigDecimal,
    pub account_status: AccountStatus,
    pub last_transaction_hash: Option<String>,
    pub last_transaction_lt: Option<String>,
    pub sync_u_time: i64,
}
