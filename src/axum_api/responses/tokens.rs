use std::str::FromStr;

use bigdecimal::BigDecimal;
use nekoton_utils::{pack_std_smc_addr, TrustMe};
use opg::OpgModel;
use serde::{Deserialize, Serialize};
use ton_block::MsgAddressInt;

use crate::axum_api::*;
use crate::models::*;

#[derive(Serialize, Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TokenBalanceResponse")]
pub struct TokenBalanceResponse {
    pub status: TonStatus,
    pub data: Option<Vec<TokenBalanceDataResponse>>,
    pub error_message: Option<String>,
}

impl From<Result<Vec<TokenBalanceDataResponse>, Error>> for TokenBalanceResponse {
    fn from(r: Result<Vec<TokenBalanceDataResponse>, Error>) -> Self {
        match r {
            Ok(data) => Self {
                status: TonStatus::Ok,
                error_message: None,
                data: Some(data),
            },
            Err(e) => Self {
                status: TonStatus::Error,
                error_message: Some(e.to_string()),
                data: None,
            },
        }
    }
}

#[derive(Serialize, Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TokenBalanceDataResponse")]
pub struct TokenBalanceDataResponse {
    pub service_id: ServiceId,
    pub address: Account,
    #[opg("balance", string)]
    pub balance: BigDecimal,
    #[opg("networkBalance", string)]
    pub network_balance: BigDecimal,
    pub account_status: AccountStatus,
    pub root_address: String,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub created_at: i64,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub updated_at: i64,
}

impl TokenBalanceDataResponse {
    pub fn new(a: TokenBalanceFromDb, b: NetworkTokenAddressData) -> Self {
        let account =
            MsgAddressInt::from_str(&format!("{}:{}", a.account_workchain_id, a.account_hex))
                .trust_me();
        let base64url = Address(pack_std_smc_addr(true, &account, true).trust_me());

        Self {
            service_id: a.service_id,
            address: Account {
                workchain_id: a.account_workchain_id,
                hex: Address(a.account_hex),
                base64url,
            },
            balance: a.balance,
            account_status: b.account_status,
            network_balance: b.network_balance,
            root_address: a.root_address,
            created_at: a.created_at.timestamp_millis(),
            updated_at: a.updated_at.timestamp_millis(),
        }
    }
}
