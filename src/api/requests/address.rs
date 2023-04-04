use opg::OpgModel;
use serde::Deserialize;

use crate::models::*;

#[derive(Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AddressCheckRequest")]
pub struct AddressCheckRequest {
    pub address: Address,
}

#[derive(Deserialize, OpgModel)]
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

#[derive(Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AddAddressRequest")]
pub struct AddAddressRequest {
    pub public_key: String,
    pub private_key: String,
    pub address: String,

    pub account_type: Option<AccountType>,
    pub custodians: Option<i32>,
    pub confirmations: Option<i32>,
    pub custodians_public_keys: Option<Vec<String>>,
}

impl From<AddAddressRequest> for AddAddress {
    fn from(v: AddAddressRequest) -> Self {
        Self {
            public_key: v.public_key,
            private_key: v.private_key,
            address: v.address,
            account_type: v.account_type,
            custodians: v.custodians,
            confirmations: v.confirmations,
            custodians_public_keys: v.custodians_public_keys,
        }
    }
}
