use opg::OpgModel;
use serde::{Deserialize, Serialize};

use crate::models::*;

#[derive(Serialize, Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AddressCheckRequest")]
pub struct AddressCheckRequest {
    pub address: Address,
}

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
