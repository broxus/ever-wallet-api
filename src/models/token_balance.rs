use bigdecimal::BigDecimal;
use tycho_types::models::StdAddr;

use crate::models::*;

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
    pub version: String,
    pub network_balance: BigDecimal,
    pub account_status: AccountStatus,
    pub last_transaction_hash: Option<String>,
    pub last_transaction_lt: Option<String>,
    pub sync_u_time: i64,
}

impl NetworkTokenAddressData {
    pub fn uninit(owner: &StdAddr, root: &StdAddr) -> NetworkTokenAddressData {
        NetworkTokenAddressData {
            workchain_id: owner.workchain as i32,
            hex: owner.address.to_string(),
            root_address: root.to_string(),
            version: Default::default(),
            account_status: AccountStatus::UnInit,
            network_balance: Default::default(),
            last_transaction_hash: None,
            last_transaction_lt: None,
            sync_u_time: 0,
        }
    }
}
