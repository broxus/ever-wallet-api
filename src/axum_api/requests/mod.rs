use http::StatusCode;
use opg::{Components, Model, OpgModel};
use serde::{Deserialize, Serialize};

use crate::models::{AccountType, CreateAddress};

#[derive(Serialize, Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("CreateAddressRequest")]
pub struct CreateAddressRequest {
    pub account_type: Option<AccountType>,
    pub workchain_id: Option<i32>,
    pub custodians: Option<i32>,
    pub confirmations: Option<i32>,
    pub custodians_public_keys: Option<Vec<String>>,
}

impl From<CreateAddressRequest> for CreateAddress {
    fn from(c: CreateAddressRequest) -> Self {
        CreateAddress {
            account_type: c.account_type,
            workchain_id: c.workchain_id,
            custodians: c.custodians,
            confirmations: c.confirmations,
            custodians_public_keys: c.custodians_public_keys,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AuthorizedError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Deserialize error")]
    DeserializeError,
}

impl AuthorizedError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            AuthorizedError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AuthorizedError::DeserializeError => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}
