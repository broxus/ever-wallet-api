use std::str::FromStr;

use bigdecimal::BigDecimal;
use nekoton_utils::pack_std_smc_addr;
use serde::{Deserialize, Serialize};
use ton_block::MsgAddressInt;
use uuid::Uuid;

use crate::models::*;
use crate::prelude::*;

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("MarkEventsResponse")]
pub struct MarkEventsResponse {
    pub status: TonStatus,
    pub error_message: Option<String>,
}

impl From<Result<TransactionEventDb, ServiceError>> for MarkEventsResponse {
    fn from(r: Result<TransactionEventDb, ServiceError>) -> Self {
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

impl From<Result<Vec<TransactionEventDb>, ServiceError>> for MarkEventsResponse {
    fn from(r: Result<Vec<TransactionEventDb>, ServiceError>) -> Self {
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
#[opg("TonTransactionsResponse")]
pub struct TonTransactionsResponse {
    pub status: TonStatus,
    pub data: Option<TransactionsResponse>,
    pub error_message: Option<String>,
}

impl From<Result<TransactionsResponse, ServiceError>> for TonTransactionsResponse {
    fn from(r: Result<TransactionsResponse, ServiceError>) -> Self {
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
    pub items: Vec<AccountTransactionEvent>,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TransactionsResponse")]
pub struct TransactionsResponse {
    pub count: i32,
    pub items: Vec<AccountTransactionDataResponse>,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TokenEventsResponse")]
pub struct TokenEventsResponse {
    pub count: i32,
    pub items: Vec<AccountTransactionEvent>,
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
#[opg("AccountTransactionEventResponse")]
pub struct AccountTransactionEventResponse {
    pub status: TonStatus,
    pub data: Option<AccountTransactionEvent>,
    pub error_message: Option<String>,
}

impl From<Result<AccountTransactionEvent, ServiceError>> for AccountTransactionEventResponse {
    fn from(r: Result<AccountTransactionEvent, ServiceError>) -> Self {
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
    pub transaction_timeout: Option<i64>,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub transaction_timestamp: Option<i64>,
    pub account: AddressResponse,
    pub sender: Option<AddressResponse>,
    pub data: Option<AccountTransactionData>,
    #[opg("value", string)]
    pub value: Option<BigDecimal>,
    #[opg("originalValue", string)]
    pub original_value: Option<BigDecimal>,
    #[opg("fee", string)]
    pub fee: Option<BigDecimal>,
    #[opg("balanceСhange", string)]
    pub balance_change: BigDecimal,
    pub out_messages: Option<Vec<AccountTransactionMessage>>,
    pub original_outputs: Option<Vec<AccountTransactionOutput>>,
    pub direction: TonTransactionDirection,
    pub status: TonTransactionStatus,
    pub aborted: bool,
    pub bounce: bool,
    pub error: Option<String>,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub created_at: i64,
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[opg("AccountTransactionData")]
pub enum AccountTransactionData {
    SafeMultisig,
    Other,
}

impl From<TransactionDb> for AccountTransactionDataResponse {
    fn from(c: TransactionDb) -> Self {
        let sender = if let (Some(sender_hex), Some(sender_workchain_id)) =
            (c.sender_hex, c.sender_workchain_id)
        {
            let sender =
                MsgAddressInt::from_str(&format!("{}:{}", sender_workchain_id, sender_hex))
                    .unwrap();
            let sender_base64url = Address(pack_std_smc_addr(true, &sender, false).unwrap());
            Some(AddressResponse {
                workchain_id: sender_workchain_id,
                hex: Address(sender_hex),
                base64url: sender_base64url,
            })
        } else {
            None
        };

        let original_outputs = if let Some(outputs) = c.original_outputs {
            let original_outputs: Vec<TransactionSendOutput> =
                serde_json::from_value(outputs).unwrap_or_default();
            Some(
                original_outputs
                    .into_iter()
                    .map(|output| {
                        let output_address =
                            nekoton_utils::repack_address(&output.recipient_address.0).unwrap();
                        let output_base64url =
                            Address(pack_std_smc_addr(true, &output_address, false).unwrap());
                        AccountTransactionOutput {
                            value: output.value.to_string(),
                            recipient: AddressResponse {
                                workchain_id: output_address.workchain_id(),
                                hex: Address(output_address.address().to_hex_string()),
                                base64url: output_base64url,
                            },
                        }
                    })
                    .collect(),
            )
        } else {
            None
        };

        let account =
            MsgAddressInt::from_str(&format!("{}:{}", c.account_workchain_id, c.account_hex))
                .unwrap();
        let base64url = Address(pack_std_smc_addr(true, &account, false).unwrap());

        AccountTransactionDataResponse {
            id: c.id,
            message_hash: c.message_hash,
            transaction_hash: c.transaction_hash,
            transaction_lt: c.transaction_lt.map(|v| v.to_string()),
            transaction_timeout: c.transaction_timeout,
            account: AddressResponse {
                workchain_id: c.account_workchain_id,
                hex: Address(c.account_hex),
                base64url,
            },
            sender,
            data: None,
            value: c.value,
            original_value: c.original_value,
            fee: c.fee,
            balance_change: c.balance_change.unwrap_or_default(),
            out_messages: c.messages.and_then(|m| serde_json::from_value(m).ok()),
            original_outputs,
            direction: c.direction,
            status: c.status,
            aborted: c.aborted,
            bounce: c.bounce,
            transaction_timestamp: c.transaction_timestamp.map(|t| t.timestamp_millis()),
            created_at: c.created_at.timestamp_millis(),
            updated_at: c.updated_at.timestamp_millis(),
            error: c.error,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountTransactionMessage")]
pub struct AccountTransactionMessage {
    pub message_hash: String,
    pub value: String,
    pub fee: String,
    pub recipient: AddressResponse,
}

#[derive(Debug, Deserialize, Serialize, Clone, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountTransactionOutput")]
pub struct AccountTransactionOutput {
    pub value: String,
    pub recipient: AddressResponse,
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
#[opg("MetricsResponse")]
pub struct MetricsResponse {
    pub gen_utime: u32,
}

impl From<Metrics> for MetricsResponse {
    fn from(r: Metrics) -> Self {
        Self {
            gen_utime: r.gen_utime,
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
            Err(_) => Self {
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
            Err(_) => Self {
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
