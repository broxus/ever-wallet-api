use opg::{Components, Model, OpgModel};
use serde::{Deserialize, Serialize};

use crate::models::{Account, TonStatus};

#[derive(Debug, Serialize, Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AddressResponse")]
pub struct AddressResponse {
    pub status: TonStatus,
    pub data: Option<Account>,
    pub error_message: Option<String>,
}

impl From<anyhow::Result<Account>> for AddressResponse {
    fn from(r: anyhow::Result<Account>) -> Self {
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
