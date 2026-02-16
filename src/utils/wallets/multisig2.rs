use std::sync::Arc;

use tycho_types::{
    abi::{AbiType, FromAbi, Function, IntoAbi, NamedAbiType, WithAbiType},
    cell::{Cell, HashBytes},
    models::StdAddr,
};

use crate::utils::{declare_function, FromAbiPlain, IntoAbiPlain, WithAbiTypePlain};

pub fn constructor() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "constructor",
        inputs: vec![
            AbiType::Array(Arc::new(AbiType::Uint(256))).named("owners"),
            AbiType::Uint(8).named("reqConfirms"),
            AbiType::Uint(32).named("lifetime"),
        ],
        outputs: vec![] as Vec<NamedAbiType>,
    }
}

pub fn send_transaction() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "sendTransaction",
        inputs: vec![
            AbiType::Address.named("dest"),
            AbiType::Uint(128).named("value"),
            AbiType::Bool.named("bounce"),
            AbiType::Uint(8).named("flags"),
            AbiType::Cell.named("payload"),
        ],
        outputs: vec![] as Vec<NamedAbiType>,
    }
}

pub fn submit_transaction() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "submitTransaction",
        inputs: vec![
            AbiType::Address.named("dest"),
            AbiType::Uint(128).named("value"),
            AbiType::Bool.named("bounce"),
            AbiType::Bool.named("allBalance"),
            AbiType::Cell.named("payload"),
            AbiType::Optional(Arc::new(AbiType::Cell)).named("stateInit"),
        ],
        outputs: vec![
            AbiType::Uint(64).named("transId")
        ],
    }
}

pub fn confirm_transaction() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "confirmTransaction",
        inputs: vec![
            AbiType::Uint(64).named("transactionId"),
            ],
        outputs: vec![] as Vec<NamedAbiType>,
    }
}

#[allow(unused)]
#[derive(Debug, WithAbiType)]
pub struct MultisigTransaction {
    pub id: u64,
    pub confirmation_mask: u32,
    pub signs_required: u8,
    pub signs_received: u8,
    pub creator: HashBytes,
    pub index: u8,
    pub dest: StdAddr,
    pub value: u128,
    pub send_flags: u16,
    pub payload: Cell,
    pub bounce: bool,
    pub state_init: Option<Cell>,
}

pub fn get_transactions() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "getTransactions",
        inputs: vec![] as Vec<NamedAbiType>,
        outputs: vec![
            AbiType::Array(Arc::new(MultisigTransaction::abi_type())).named("transactions")
        ]
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, WithAbiType)]
pub struct MultisigCustodian {
    pub index: u8,
    pub pubkey: HashBytes,
}

pub fn get_custodians() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "getCustodians",
        inputs: vec![] as Vec<NamedAbiType>,
        outputs: vec![
            AbiType::Array(Arc::new(MultisigCustodian::abi_type())).named("custodians")
        ]
    }
}

#[derive(Debug, Clone, WithAbiType, FromAbi, IntoAbi)]
pub struct SubmitUpdateParams {
    pub code_hash: Option<HashBytes>,
    pub owners: Option<Vec<HashBytes>>,
    pub req_confirms: Option<u8>,
    pub lifetime: Option<u64>,
}
impl IntoAbiPlain for SubmitUpdateParams {}
impl FromAbiPlain for SubmitUpdateParams {}
impl WithAbiTypePlain for SubmitUpdateParams {}

#[derive(Debug, Copy, Clone, WithAbiType, FromAbi, IntoAbi)]
pub struct SubmitUpdateOutput {
    pub update_id: u64,
}
impl IntoAbiPlain for SubmitUpdateOutput {}
impl FromAbiPlain for SubmitUpdateOutput {}
impl WithAbiTypePlain for SubmitUpdateOutput {}

pub fn submit_update() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "submitUpdate",
        inputs: SubmitUpdateParams::abi_type_plain(),
        outputs: SubmitUpdateOutput::abi_type_plain(),
    }
}

#[derive(Debug, Copy, Clone, WithAbiType, FromAbi, IntoAbi)]
pub struct ConfirmUpdateParams {
    pub update_id: u64,
}
impl IntoAbiPlain for ConfirmUpdateParams {}
impl FromAbiPlain for ConfirmUpdateParams {}
impl WithAbiTypePlain for ConfirmUpdateParams {}

pub fn confirm_update() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "confirmUpdate",
        inputs: ConfirmUpdateParams::abi_type_plain(),
        outputs: vec![] as Vec<NamedAbiType>,
    }
}

#[derive(Debug, Clone, WithAbiType, FromAbi, IntoAbi)]
pub struct ExecuteUpdateParams {
    pub update_id: u64,
    pub code: Option<Cell>,
}
impl IntoAbiPlain for ExecuteUpdateParams {}
impl FromAbiPlain for ExecuteUpdateParams {}
impl WithAbiTypePlain for ExecuteUpdateParams {}

pub fn execute_update() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "executeUpdate",
        inputs: ExecuteUpdateParams::abi_type_plain(),
        outputs: vec![] as Vec<NamedAbiType>,
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub struct SetCodeMultisigParams {
    pub max_queued_transactions: u8,
    pub max_custodian_count: u8,
    pub expiration_time: u64,
    pub min_value: u128,
    pub required_txn_confirms: u8,
    pub required_upd_confirms: u8,
}

impl SetCodeMultisigParams {
    fn abi_type() -> Vec<NamedAbiType> {
        vec![
            AbiType::Uint(8).named("maxQueuedTransactions"),
            AbiType::Uint(8).named("maxCustodianCount"),
            AbiType::Uint(64).named("expirationTime"),
            AbiType::Uint(128).named("minValue"),
            AbiType::Uint(8).named("requiredTxnConfirms"),
            AbiType::Uint(8).named("requiredUpdConfirms"),
        ]
    }
}
pub fn get_parameters() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "getParameters",
        inputs: vec![] as Vec<NamedAbiType>,
        outputs: SetCodeMultisigParams::abi_type(),
    }
}

#[derive(IntoAbi, FromAbi, WithAbiType, Eq, PartialEq, Debug, Clone)]
pub struct UpdateTransaction {
    pub id: u64,
    pub index: u8,
    pub signs: u8,
    #[abi(name = "confirmationsMask")]
    pub confirmations_mask: u32,
    pub creator: HashBytes,
    #[abi(name = "newCodeHash")]
    pub new_code_hash: Option<HashBytes>,
    #[abi(name = "newCustodians")]
    pub new_custodians: Option<Vec<HashBytes>>,
    #[abi(name = "newReqConfirms")]
    pub new_req_confirms: Option<u8>,
    #[abi(name = "newLifetime")]
    pub new_lifetime: Option<u32>,
}

pub mod v2_0 {
    use tycho_types::abi::NamedAbiType;

    use super::*;

    pub fn get_update_requests() -> &'static Function {
        declare_function! {
            abi: v2_3,
            header: [pubkey, time, expire],
            name: "getUpdateRequests",
            inputs: vec![] as Vec<NamedAbiType>,
            outputs: {
                let param_types = match UpdateTransaction::abi_type() {
                    AbiType::Tuple(params) => {
                        let mut vec = params.iter().cloned().collect::<Vec<_>>();
                        if let Some(last) = vec.last_mut() {
                            if let NamedAbiType { ty: AbiType::Optional(param), name } = last {
                                if let AbiType::Uint(_) = param.as_ref() {
                                    *last = AbiType::Optional(Arc::new(AbiType::Uint(64))).named(&*name.clone());
                                }
                            }
                        }
                        AbiType::Tuple(Arc::<[NamedAbiType]>::from(vec))
                    }
                    other => other,
                };

                vec![
                    AbiType::Array(Arc::new(param_types)).named("updates")
                ]
            },
        }
    }
}

pub mod v2_1 {
    use super::*;

    pub fn get_update_requests() -> &'static Function {
        declare_function! {
            abi: v2_3,
            header: [pubkey, time, expire],
            name: "getUpdateRequests",
            inputs: vec![] as Vec<NamedAbiType>,
            outputs: vec![
                AbiType::Array(Arc::new(UpdateTransaction::abi_type())).named("updates")
            ],
        }
    }
}
