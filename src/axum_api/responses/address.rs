use std::str::FromStr;

use anyhow::Result;
use bigdecimal::BigDecimal;
use derive_more::Constructor;
use nekoton_utils::{pack_std_smc_addr, TrustMe};
use opg::OpgModel;
use serde::{Deserialize, Serialize};
use ton_block::MsgAddressInt;
use uuid::Uuid;

use crate::models::*;

#[derive(Serialize, Deserialize, OpgModel, Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("AddressValidResponse")]
pub struct AddressValidResponse {
    pub valid: bool,
}

#[derive(Serialize, Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AddressResponse")]
pub struct AddressResponse {
    pub status: TonStatus,
    pub data: Option<Account>,
    pub error_message: Option<String>,
}

impl From<Result<Account>> for AddressResponse {
    fn from(r: Result<Account>) -> Self {
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

#[derive(Serialize, Deserialize, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("CheckedAddressResponse")]
pub struct CheckedAddressResponse {
    pub status: TonStatus,
    pub data: Option<AddressValidResponse>,
}

impl From<Result<AddressValidResponse>> for CheckedAddressResponse {
    fn from(r: Result<AddressValidResponse>) -> Self {
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

#[derive(Serialize, Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AddressBalanceResponse")]
pub struct AddressBalanceResponse {
    pub status: TonStatus,
    pub data: Option<AddressBalanceDataResponse>,
    pub error_message: Option<String>,
}

impl From<Result<AddressBalanceDataResponse>> for AddressBalanceResponse {
    fn from(r: Result<AddressBalanceDataResponse>) -> Self {
        match r {
            Ok(data) => Self {
                status: TonStatus::Ok,
                data: Some(data),
                error_message: None,
            },
            Err(e) => Self {
                status: TonStatus::Error,
                data: None,
                error_message: Some(format!("{:?}", e)),
            },
        }
    }
}

#[derive(Serialize, Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AddressInfoResponse")]
pub struct AddressInfoResponse {
    pub status: TonStatus,
    pub data: Option<AddressInfoDataResponse>,
    pub error_message: Option<String>,
}

impl From<Result<AddressInfoDataResponse>> for AddressInfoResponse {
    fn from(r: Result<AddressInfoDataResponse>) -> Self {
        match r {
            Ok(data) => Self {
                status: TonStatus::Ok,
                data: Some(data),
                error_message: None,
            },
            Err(e) => Self {
                status: TonStatus::Error,
                data: None,
                error_message: Some(format!("{:?}", e)),
            },
        }
    }
}

#[derive(Serialize, Deserialize, OpgModel)]
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

#[derive(Serialize, Deserialize, OpgModel)]
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
