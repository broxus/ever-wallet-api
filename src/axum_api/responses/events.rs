use opg::OpgModel;
use serde::Serialize;

use crate::axum_api::*;
use crate::models::*;

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("EventsResponse")]
pub struct EventsResponse {
    pub count: i32,
    pub items: Vec<AccountTransactionEvent>,
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TonEventsResponse")]
pub struct TonEventsResponse {
    pub status: TonStatus,
    pub data: Option<EventsResponse>,
    pub error_message: Option<String>,
}

impl From<Result<EventsResponse, Error>> for TonEventsResponse {
    fn from(r: Result<EventsResponse, Error>) -> Self {
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

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("MarkEventsResponse")]
pub struct MarkEventsResponse {
    pub status: TonStatus,
    pub error_message: Option<String>,
}

impl From<Result<TransactionEventDb, Error>> for MarkEventsResponse {
    fn from(r: Result<TransactionEventDb, Error>) -> Self {
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

impl From<Result<Vec<TransactionEventDb>, Error>> for MarkEventsResponse {
    fn from(r: Result<Vec<TransactionEventDb>, Error>) -> Self {
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

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TonEventsResponse")]
pub struct TonTokenEventsResponse {
    pub status: TonStatus,
    pub data: Option<TokenEventsResponse>,
    pub error_message: Option<String>,
}

impl From<Result<TokenEventsResponse, Error>> for TonTokenEventsResponse {
    fn from(r: Result<TokenEventsResponse, Error>) -> Self {
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

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TokenEventsResponse")]
pub struct TokenEventsResponse {
    pub count: i32,
    pub items: Vec<AccountTransactionEvent>,
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("MarkTokenEventsResponse")]
pub struct MarkTokenEventsResponse {
    pub status: TonStatus,
    pub error_message: Option<String>,
}

impl From<Result<TokenTransactionEventDb, Error>> for MarkTokenEventsResponse {
    fn from(r: Result<TokenTransactionEventDb, Error>) -> Self {
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

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TransactionEventResponse")]
pub struct TransactionEventResponse {
    pub status: TonStatus,
    pub data: Option<AccountTransactionEvent>,
    pub error_message: Option<String>,
}

impl From<Result<AccountTransactionEvent, Error>> for TransactionEventResponse {
    fn from(r: Result<AccountTransactionEvent, Error>) -> Self {
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
