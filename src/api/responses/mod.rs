use std::str::FromStr;

use bigdecimal::BigDecimal;
use nekoton_utils::pack_std_smc_addr;
use serde::{Deserialize, Serialize};
use ton_block::MsgAddressInt;
use uuid::Uuid;

use crate::models::account_enums::{
    AccountStatus, AccountType, AddressResponse, TonEventStatus, TonStatus,
    TonTokenTransactionStatus, TonTransactionDirection, TonTransactionStatus,
};
use crate::models::address::{Address, NetworkAddressData};
use crate::models::service_id::ServiceId;
use crate::models::sqlx::{
    AddressDb, TokenBalanceFromDb, TokenTransactionEventDb, TokenTransactionFromDb, TransactionDb,
    TransactionEventDb,
};
use crate::models::token_balance::NetworkTokenAddressData;
use crate::prelude::ServiceError;

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("MarkEventsResponse")]
pub struct MarkEventsResponse {
    pub status: TonStatus,
    pub error_message: Option<String>,
}

impl From<Result<(), ServiceError>> for MarkEventsResponse {
    fn from(r: Result<(), ServiceError>) -> Self {
        match r {
            Ok(_) => Self {
                status: TonStatus::Ok,
                error_message: None,
            },
            Err(e) => Self {
                status: TonStatus::Error,
                error_message: Some(e.to_string()),
            },
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("MarkTokenEventsResponse")]
pub struct MarkTokenEventsResponse {
    pub status: TonStatus,
    pub error_message: Option<String>,
}

impl From<Result<(), ServiceError>> for MarkTokenEventsResponse {
    fn from(r: Result<(), ServiceError>) -> Self {
        match r {
            Ok(_) => Self {
                status: TonStatus::Ok,
                error_message: None,
            },
            Err(e) => Self {
                status: TonStatus::Error,
                error_message: Some(e.to_string()),
            },
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TonEventsResponse")]
pub struct TonEventsResponse {
    pub status: TonStatus,
    pub data: Option<EventsResponse>,
    pub error_message: Option<String>,
}

impl From<Result<EventsResponse, ServiceError>> for TonEventsResponse {
    fn from(r: Result<EventsResponse, ServiceError>) -> Self {
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

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TonEventsResponse")]
pub struct TonTokenEventsResponse {
    pub status: TonStatus,
    pub data: Option<TokenEventsResponse>,
    pub error_message: Option<String>,
}

impl From<Result<TokenEventsResponse, ServiceError>> for TonTokenEventsResponse {
    fn from(r: Result<TokenEventsResponse, ServiceError>) -> Self {
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

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("EventsResponse")]
pub struct EventsResponse {
    pub count: i32,
    pub items: Vec<AccountTransactionEventResponse>,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TokenEventsResponse")]
pub struct TokenEventsResponse {
    pub count: i32,
    pub items: Vec<AccountTokenTransactionEventResponse>,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountTransactionEventResponse")]
pub struct AccountTransactionEventResponse {
    pub id: Uuid,
    pub service_id: ServiceId,
    pub transaction_id: Uuid,
    pub message_hash: String,
    pub account_workchain_id: i32,
    pub account_hex: String,
    #[opg("balanceChange", string, optional)]
    pub balance_change: Option<BigDecimal>,
    pub transaction_direction: TonTransactionDirection,
    pub transaction_status: TonTransactionStatus,
    pub event_status: TonEventStatus,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub created_at: i64,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub updated_at: i64,
}

impl From<TransactionEventDb> for AccountTransactionEventResponse {
    fn from(c: TransactionEventDb) -> Self {
        AccountTransactionEventResponse {
            id: c.id,
            service_id: c.service_id,
            transaction_id: c.transaction_id,
            message_hash: c.message_hash,
            account_workchain_id: c.account_workchain_id,
            account_hex: c.account_hex,
            balance_change: c.balance_change,
            transaction_direction: c.transaction_direction,
            transaction_status: c.transaction_status,
            event_status: c.event_status,
            created_at: c.created_at.timestamp_millis(),
            updated_at: c.updated_at.timestamp_millis(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountTokenTransactionEventResponse")]
pub struct AccountTokenTransactionEventResponse {
    pub id: Uuid,
    pub service_id: ServiceId,
    pub token_transaction_id: Uuid,
    pub message_hash: String,
    pub account_workchain_id: i32,
    pub account_hex: String,
    #[opg("value", string)]
    pub value: BigDecimal,
    pub root_address: String,
    pub transaction_direction: TonTransactionDirection,
    pub transaction_status: TonTokenTransactionStatus,
    pub event_status: TonEventStatus,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub created_at: i64,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub updated_at: i64,
}

impl From<TokenTransactionEventDb> for AccountTokenTransactionEventResponse {
    fn from(c: TokenTransactionEventDb) -> Self {
        AccountTokenTransactionEventResponse {
            id: c.id,
            service_id: c.service_id,
            token_transaction_id: c.token_transaction_id,
            message_hash: c.message_hash,
            account_workchain_id: c.account_workchain_id,
            account_hex: c.account_hex,
            value: c.value,
            root_address: c.root_address,
            transaction_direction: c.transaction_direction,
            transaction_status: c.transaction_status,
            event_status: c.event_status,
            created_at: c.created_at.timestamp_millis(),
            updated_at: c.updated_at.timestamp_millis(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TokenBalanceResponse")]
pub struct TokenBalanceResponse {
    pub service_id: ServiceId,
    pub address: AddressResponse,
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

impl TokenBalanceResponse {
    pub fn new(a: TokenBalanceFromDb, b: NetworkTokenAddressData) -> Self {
        let account =
            MsgAddressInt::from_str(&format!("{}:{}", a.account_workchain_id, a.account_hex))
                .unwrap();
        let base64url = Address(pack_std_smc_addr(true, &account, false).unwrap());

        Self {
            service_id: a.service_id,
            address: AddressResponse {
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

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountAddressResponse")]
pub struct AccountAddressResponse {
    pub status: TonStatus,
    pub data: Option<AddressResponse>,
    pub error_message: Option<String>,
}

impl From<Result<AddressResponse, ServiceError>> for AccountAddressResponse {
    fn from(r: Result<AddressResponse, ServiceError>) -> Self {
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

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountAddressResponse")]
pub struct AccountTokenBalanceResponse {
    pub status: TonStatus,
    pub data: Option<Vec<TokenBalanceResponse>>,
    pub error_message: Option<String>,
}

impl From<Result<Vec<TokenBalanceResponse>, ServiceError>> for AccountTokenBalanceResponse {
    fn from(r: Result<Vec<TokenBalanceResponse>, ServiceError>) -> Self {
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

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountTransactionResponse")]
pub struct AccountTransactionResponse {
    pub status: TonStatus,
    pub data: Option<AccountTransactionDataResponse>,
    pub error_message: Option<String>,
}

impl From<Result<AccountTransactionDataResponse, ServiceError>> for AccountTransactionResponse {
    fn from(r: Result<AccountTransactionDataResponse, ServiceError>) -> Self {
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

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountTransactionsResponse")]
pub struct AccountTransactionsResponse {
    pub count: i32,
    pub items: Vec<AccountTransactionDataResponse>,
}

impl AccountTransactionsResponse {
    pub fn new(ts: Vec<TransactionDb>) -> Self {
        Self {
            count: ts.len() as i32,
            items: ts.into_iter().map(From::from).collect(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountTransactionsResponse")]
pub struct AccountTransactionsDataResponse {
    pub status: TonStatus,
    pub data: Option<AccountTransactionsResponse>,
    pub error_message: Option<String>,
}

impl From<Result<AccountTransactionsResponse, ServiceError>> for AccountTransactionsDataResponse {
    fn from(r: Result<AccountTransactionsResponse, ServiceError>) -> Self {
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

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountTransactionDataResponse")]
pub struct AccountTransactionDataResponse {
    #[opg("id", string)]
    pub id: Uuid,
    pub message_hash: String,
    pub transaction_hash: Option<String>,
    pub transaction_lt: Option<String>,
    pub account: AddressResponse,
    #[opg("value", string)]
    pub value: Option<BigDecimal>,
    #[opg("balanceСhange", string)]
    pub balance_change: BigDecimal,
    pub direction: TonTransactionDirection,
    pub status: TonTransactionStatus,
    pub aborted: bool,
    pub bounce: bool,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub created_at: i64,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub updated_at: i64,
}

impl From<TransactionDb> for AccountTransactionDataResponse {
    fn from(c: TransactionDb) -> Self {
        let account =
            MsgAddressInt::from_str(&format!("{}:{}", c.account_workchain_id, c.account_hex))
                .unwrap();
        let base64url = Address(pack_std_smc_addr(true, &account, false).unwrap());

        AccountTransactionDataResponse {
            id: c.id,
            message_hash: c.message_hash,
            transaction_hash: c.transaction_hash,
            transaction_lt: c.transaction_lt.map(|v| v.to_string()),
            account: AddressResponse {
                workchain_id: c.account_workchain_id,
                hex: Address(c.account_hex),
                base64url,
            },
            value: c.value,
            balance_change: c.balance_change.unwrap_or_default(),
            direction: c.direction,
            status: c.status,
            aborted: c.aborted,
            bounce: c.bounce,
            created_at: c.created_at.timestamp_millis(),
            updated_at: c.updated_at.timestamp_millis(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountTokenTransactionResponse")]
pub struct AccountTokenTransactionResponse {
    pub status: TonStatus,
    pub data: Option<AccountTokenTransactionDataResponse>,
    pub error_message: Option<String>,
}

impl From<Result<AccountTokenTransactionDataResponse, ServiceError>>
    for AccountTokenTransactionResponse
{
    fn from(r: Result<AccountTokenTransactionDataResponse, ServiceError>) -> Self {
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

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountTokenTransactionDataResponse")]
pub struct AccountTokenTransactionDataResponse {
    pub id: Uuid,
    pub transaction_hash: Option<String>,
    pub message_hash: String,
    pub account: AddressResponse,
    #[opg("value", string)]
    pub value: BigDecimal,
    pub root_address: String,
    pub error: Option<String>,
    pub block_hash: Option<String>,
    pub block_time: Option<i32>,
    pub direction: TonTransactionDirection,
    pub status: TonTokenTransactionStatus,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub created_at: i64,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub updated_at: i64,
}

impl From<TokenTransactionFromDb> for AccountTokenTransactionDataResponse {
    fn from(c: TokenTransactionFromDb) -> Self {
        let account =
            MsgAddressInt::from_str(&format!("{}:{}", c.account_workchain_id, c.account_hex))
                .unwrap();
        let base64url = Address(pack_std_smc_addr(true, &account, false).unwrap());

        AccountTokenTransactionDataResponse {
            id: c.id,
            message_hash: c.message_hash,
            transaction_hash: c.transaction_hash,
            account: AddressResponse {
                workchain_id: c.account_workchain_id,
                hex: Address(c.account_hex),
                base64url,
            },
            value: c.value,
            root_address: c.root_address,
            error: c.error,
            block_hash: c.block_hash,
            block_time: c.block_time,
            direction: c.direction,
            status: c.status,
            created_at: c.created_at.timestamp_millis(),
            updated_at: c.updated_at.timestamp_millis(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel, derive_more::Constructor)]
#[serde(rename_all = "camelCase")]
#[opg("PostAddressValidResponse")]
pub struct PostAddressValidResponse {
    pub valid: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostCheckedAddressResponse")]
pub struct PostCheckedAddressResponse {
    pub status: TonStatus,
    pub data: Option<PostAddressValidResponse>,
}

impl From<Result<PostAddressValidResponse, ServiceError>> for PostCheckedAddressResponse {
    fn from(r: Result<PostAddressValidResponse, ServiceError>) -> Self {
        match r {
            Ok(data) => Self {
                status: TonStatus::Ok,
                data: Some(data),
            },
            Err(e) => Self {
                status: TonStatus::Error,
                data: None,
            },
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AddressBalanceResponse")]
pub struct AddressBalanceResponse {
    pub status: TonStatus,
    pub data: Option<PostAddressBalanceDataResponse>,
}

impl From<Result<PostAddressBalanceDataResponse, ServiceError>> for AddressBalanceResponse {
    fn from(r: Result<PostAddressBalanceDataResponse, ServiceError>) -> Self {
        match r {
            Ok(data) => Self {
                status: TonStatus::Ok,
                data: Some(data),
            },
            Err(e) => Self {
                status: TonStatus::Error,
                data: None,
            },
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("PostAddressBalanceDataResponse")]
pub struct PostAddressBalanceDataResponse {
    pub id: Uuid,
    pub address: AddressResponse,
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

impl PostAddressBalanceDataResponse {
    pub fn new(a: AddressDb, b: NetworkAddressData) -> Self {
        let account = MsgAddressInt::from_str(&format!("{}:{}", a.workchain_id, a.hex)).unwrap();
        let base64url = Address(pack_std_smc_addr(true, &account, false).unwrap());

        Self {
            id: a.id,
            address: AddressResponse {
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
