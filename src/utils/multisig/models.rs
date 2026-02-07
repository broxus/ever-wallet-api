use serde::{Deserialize, Serialize};
use tycho_types::{
    cell::{Cell, HashBytes},
    models::StdAddr,
};

use crate::utils::{serde_address, serde_cell, serde_string};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "data")]
pub enum MultisigTransaction {
    Send(MultisigSendTransaction),
    Submit(MultisigSubmitTransaction),
    Confirm(MultisigConfirmTransaction),
    SubmitUpdate(MultisigSubmitUpdate),
    ConfirmUpdate(MultisigConfirmUpdate),
    ExecuteUpdate(MultisigExecuteUpdate),
}

#[derive(UnpackAbiPlain, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
#[serde(rename_all = "camelCase")]
pub struct MultisigConfirmTransaction {
    #[abi(skip)]
    #[serde(with = "serde_HashBytes")]
    pub custodian: HashBytes,

    #[abi(uint64, name = "transactionId")]
    #[serde(with = "serde_string")]
    pub transaction_id: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultisigSubmitTransaction {
    #[serde(with = "serde_HashBytes")]
    pub custodian: HashBytes,

    #[serde(with = "serde_address")]
    pub dest: StdAddr,

    #[serde(with = "serde_string")]
    pub value: u128,

    pub bounce: bool,

    pub all_balance: bool,

    #[serde(with = "serde_cell")]
    pub payload: Cell,

    #[serde(with = "serde_string")]
    pub trans_id: u64,
}

#[derive(UnpackAbiPlain, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MultisigSendTransaction {
    #[abi(address)]
    #[serde(with = "serde_address")]
    pub dest: StdAddr,

    #[abi(with = "nekoton_abi::uint128_number")]
    #[serde(with = "serde_string")]
    pub value: u128,

    #[abi(bool)]
    pub bounce: bool,

    #[abi(uint8)]
    pub flags: u8,

    #[abi(cell)]
    #[serde(with = "serde_cell")]
    pub payload: Cell,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MultisigSubmitUpdate {
    #[serde(with = "serde_HashBytes")]
    pub custodian: HashBytes,
    #[serde(with = "serde_optional_HashBytes")]
    pub new_code_hash: Option<HashBytes>,
    pub new_owners: bool,
    pub new_req_confirms: bool,
    pub new_lifetime: bool,
    #[serde(with = "serde_string")]
    pub update_id: u64,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MultisigConfirmUpdate {
    #[serde(with = "serde_HashBytes")]
    pub custodian: HashBytes,
    #[serde(with = "serde_string")]
    pub update_id: u64,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MultisigExecuteUpdate {
    #[serde(with = "serde_HashBytes")]
    pub custodian: HashBytes,
    #[serde(with = "serde_string")]
    pub update_id: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct MultisigPendingTransaction {
    #[serde(with = "serde_string")]
    pub id: u64,

    #[serde(with = "serde_vec_HashBytes")]
    pub confirmations: Vec<HashBytes>,

    pub signs_required: u8,
    pub signs_received: u8,

    #[serde(with = "serde_HashBytes")]
    pub creator: HashBytes,

    pub index: u8,

    #[serde(with = "serde_address")]
    pub dest: StdAddr,

    #[serde(with = "serde_string")]
    pub value: u128,

    pub send_flags: u16,

    #[serde(with = "serde_cell")]
    pub payload: Cell,

    pub bounce: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct MultisigPendingUpdate {
    #[serde(with = "serde_string")]
    pub id: u64,

    #[serde(with = "serde_vec_HashBytes")]
    pub confirmations: Vec<HashBytes>,

    pub signs_received: u8,

    #[serde(with = "serde_HashBytes")]
    pub creator: HashBytes,

    pub index: u8,

    #[serde(with = "serde_optional_HashBytes")]
    pub new_code_hash: Option<HashBytes>,
    #[serde(with = "serde_optional_vec_HashBytes")]
    pub new_custodians: Option<Vec<HashBytes>>,
    pub new_req_confirms: Option<u8>,
    pub new_lifetime: Option<u32>,
}
