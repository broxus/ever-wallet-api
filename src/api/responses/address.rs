use std::str::FromStr;

use bigdecimal::BigDecimal;
use derive_more::Constructor;
use nekoton_utils::{pack_std_smc_addr, TrustMe};
use opg::OpgModel;
use serde::Serialize;
use ton_block::MsgAddressInt;
use uuid::Uuid;

use crate::api::*;
use crate::models::*;

#[derive(Serialize, OpgModel, Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("AddressValidResponse")]
pub struct AddressValidResponse {
    pub valid: bool,
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AddressResponse")]
pub struct AddressResponse {
    pub status: TonStatus,
    pub data: Option<Account>,
    pub error_message: Option<String>,
}

impl From<Result<Account, Error>> for AddressResponse {
    fn from(r: Result<Account, Error>) -> Self {
        match r {
            Ok(data) => Self {
                status: TonStatus::Ok,
                error_message: None,
                data: Some(data),
            },
            Err(e) => Self {
                status: TonStatus::Error,
                error_message: Some(e.get_error()),
                data: None,
            },
        }
    }
}

#[derive(Serialize, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("CheckedAddressResponse")]
pub struct CheckedAddressResponse {
    pub status: TonStatus,
    pub data: Option<AddressValidResponse>,
}

impl From<Result<AddressValidResponse, Error>> for CheckedAddressResponse {
    fn from(r: Result<AddressValidResponse, Error>) -> Self {
        match r {
            Ok(data) => Self {
                status: TonStatus::Ok,
                data: Some(data),
            },
            Err(_) => Self {
                status: TonStatus::Error,
                data: None,
            },
        }
    }
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AddressBalanceResponse")]
pub struct AddressBalanceResponse {
    pub status: TonStatus,
    pub data: Option<AddressBalanceDataResponse>,
    pub error_message: Option<String>,
}

impl From<Result<AddressBalanceDataResponse, Error>> for AddressBalanceResponse {
    fn from(r: Result<AddressBalanceDataResponse, Error>) -> Self {
        match r {
            Ok(data) => Self {
                status: TonStatus::Ok,
                data: Some(data),
                error_message: None,
            },
            Err(e) => Self {
                status: TonStatus::Error,
                data: None,
                error_message: Some(e.get_error()),
            },
        }
    }
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AddressBalanceDataResponse")]
pub struct AddressBalanceDataResponse {
    pub id: Uuid,
    pub address: Account,
    pub account_type: AccountType,
    pub account_status: AccountStatus,
    #[opg("balance", string)]
    pub balance: BigDecimal,
    #[opg("networkBalance", string)]
    pub network_balance: BigDecimal,
    pub last_transaction_hash: Option<String>,
    pub last_transaction_lt: Option<String>,
    pub sync_u_time: i64,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub created_at: i64,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub updated_at: i64,
}

impl AddressBalanceDataResponse {
    pub fn new(a: AddressDb, b: NetworkAddressData) -> Self {
        let account = MsgAddressInt::from_str(&format!("{}:{}", a.workchain_id, a.hex)).trust_me();
        let base64url = Address(pack_std_smc_addr(true, &account, true).trust_me());

        Self {
            id: a.id,
            address: Account {
                workchain_id: a.workchain_id,
                hex: Address(a.hex),
                base64url,
            },
            account_type: a.account_type,
            balance: a.balance,
            account_status: b.account_status,
            network_balance: b.network_balance,
            last_transaction_hash: b.last_transaction_hash,
            last_transaction_lt: b.last_transaction_lt,
            sync_u_time: b.sync_u_time,
            created_at: a.created_at.timestamp_millis(),
            updated_at: a.updated_at.timestamp_millis(),
        }
    }
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AddressInfoResponse")]
pub struct AddressInfoResponse {
    pub status: TonStatus,
    pub data: Option<AddressInfoDataResponse>,
    pub error_message: Option<String>,
}

impl From<Result<AddressInfoDataResponse, Error>> for AddressInfoResponse {
    fn from(r: Result<AddressInfoDataResponse, Error>) -> Self {
        match r {
            Ok(data) => Self {
                status: TonStatus::Ok,
                data: Some(data),
                error_message: None,
            },
            Err(e) => Self {
                status: TonStatus::Error,
                data: None,
                error_message: Some(e.get_error()),
            },
        }
    }
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AddressInfoDataResponse")]
pub struct AddressInfoDataResponse {
    pub id: Uuid,
    pub address: Account,
    pub account_type: AccountType,
    #[opg("balance", string)]
    pub balance: BigDecimal,
    pub custodians: Option<i32>,
    pub confirmations: Option<i32>,
    pub custodians_public_keys: Option<Vec<String>>,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub created_at: i64,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub updated_at: i64,
}

impl AddressInfoDataResponse {
    pub fn new(a: AddressDb) -> Self {
        let account = MsgAddressInt::from_str(&format!("{}:{}", a.workchain_id, a.hex)).trust_me();
        let base64url = Address(pack_std_smc_addr(true, &account, true).trust_me());

        Self {
            id: a.id,
            address: Account {
                workchain_id: a.workchain_id,
                hex: Address(a.hex),
                base64url,
            },
            account_type: a.account_type,
            custodians: a.custodians,
            confirmations: a.confirmations,
            custodians_public_keys: a
                .custodians_public_keys
                .and_then(|k| serde_json::from_value(k).unwrap_or_default()),
            balance: a.balance,
            created_at: a.created_at.timestamp_millis(),
            updated_at: a.updated_at.timestamp_millis(),
        }
    }
}

#[derive(Serialize, OpgModel)]
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
                error_message: Some(e.get_error()),
                data: None,
            },
        }
    }
}

#[derive(Serialize, OpgModel)]
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
