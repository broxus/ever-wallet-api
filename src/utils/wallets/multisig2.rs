use std::sync::Arc;

use tycho_types::{
    abi::{AbiType, FromAbi, Function, IntoAbi, NamedAbiType, NamedAbiValue, WithAbiType},
    cell::{Cell, HashBytes},
    models::StdAddr,
};

use crate::utils::declare_function;

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

#[derive(Debug)]
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

impl WithAbiType for MultisigTransaction {
    fn abi_type() -> AbiType {
        AbiType::Tuple(Arc::new([
            AbiType::Uint(64).named("id"),
            AbiType::Uint(32).named("confirmationMask"),
            AbiType::Uint(8).named("signsRequired"),
            AbiType::Uint(8).named("signsReceived"),
            AbiType::Uint(256).named("creator"),
            AbiType::Uint(8).named("index"),
            AbiType::Address.named("dest"),
            AbiType::Uint(128).named("value"),
            AbiType::Uint(16).named("sendFlags"),
            AbiType::Cell.named("payload"),
            AbiType::Bool.named("bounce"),
            AbiType::Optional(Arc::new(AbiType::Cell)).named("stateInit"),
        ]))
    }
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

#[derive(Debug, Clone, Copy)]
pub struct MultisigCustodian {
    pub index: u8,
    pub pubkey: HashBytes,
}

impl WithAbiType for MultisigCustodian {
    fn abi_type() -> AbiType {
        AbiType::Tuple(Arc::new([
            AbiType::Uint(8).named("index"),
            AbiType::Uint(256).named("pubkey"),
        ]))
    }
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

#[derive(Debug, Clone)]
pub struct SubmitUpdateParams {
    pub code_hash: Option<HashBytes>,
    pub owners: Option<Vec<HashBytes>>,
    pub req_confirms: Option<u8>,
    pub lifetime: Option<u64>,
}

impl SubmitUpdateParams {
    fn abi_type() -> Vec<NamedAbiType> {
        vec![
            AbiType::Optional(Arc::new(AbiType::Uint(256))).named("codeHash"),
            AbiType::Optional(Arc::new(AbiType::Array(Arc::new(AbiType::Uint(256)))))
                .named("owners"),
            AbiType::Optional(Arc::new(AbiType::Uint(8))).named("reqConfirms"),
            AbiType::Optional(Arc::new(AbiType::Uint(64))).named("lifetime"),
        ]
    }

    pub fn abi_values(&self) -> Vec<NamedAbiValue> {
        vec![
            self.code_hash.as_abi().named("codeHash"),
            self.owners.as_abi().named("owners"),
            self.req_confirms.as_abi().named("reqConfirms"),
            self.lifetime.as_abi().named("lifetime"),
        ]
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SubmitUpdateOutput {
    pub update_id: u64,
}

impl SubmitUpdateOutput {
    fn abi_type() -> Vec<NamedAbiType> {
        vec![AbiType::Uint(64).named("updateId")]
    }
}

pub fn submit_update() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "submitUpdate",
        inputs: SubmitUpdateParams::abi_type(),
        outputs: SubmitUpdateOutput::abi_type(),
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ConfirmUpdateParams {
    pub update_id: u64,
}

impl ConfirmUpdateParams {
    fn abi_type() -> Vec<NamedAbiType> {
        vec![AbiType::Uint(64).named("updateId")]
    }

    pub fn abi_values(&self) -> Vec<NamedAbiValue> {
        vec![self.update_id.as_abi().named("updateId")]
    }
}

pub fn confirm_update() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "confirmUpdate",
        inputs: ConfirmUpdateParams::abi_type(),
        outputs: vec![] as Vec<NamedAbiType>,
    }
}

#[derive(Debug, Clone)]
pub struct ExecuteUpdateParams {
    pub update_id: u64,
    pub code: Option<Cell>,
}

impl ExecuteUpdateParams {
    fn abi_type() -> Vec<NamedAbiType> {
        vec![
            AbiType::Uint(64).named("updateId"),
            AbiType::Optional(Arc::new(AbiType::Cell)).named("code"),
        ]
    }

    pub fn abi_values(&self) -> Vec<NamedAbiValue> {
        vec![
            self.update_id.as_abi().named("updateId"),
            self.code.as_abi().named("code"),
        ]
    }
}

pub fn execute_update() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "executeUpdate",
        inputs: ExecuteUpdateParams::abi_type(),
        outputs: vec![] as Vec<NamedAbiType>,
    }
}

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
