use std::sync::Arc;

use tycho_types::{
    abi::{AbiType, Function},
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
        outputs: Vec::new(),
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
        outputs: Vec::new(),
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
        outputs: Vec::new(),
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

pub fn get_transactions() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "getTransactions",
        inputs: Vec::new(),
        outputs: vec![
            AbiType::Array(Arc::new(MultisigTransaction::param_type())).named("transactions")
        ]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MultisigCustodian {
    pub index: u8,
    pub pubkey: HashBytes,
}

pub fn get_custodians() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "getCustodians",
        inputs: Vec::new(),
        outputs: vec![
            AbiType::Array(Arc::new(MultisigCustodian::param_type())).named("custodians")
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

#[derive(Debug, Copy, Clone)]
pub struct SubmitUpdateOutput {
    pub update_id: u64,
}

pub fn submit_update() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "submitUpdate",
        inputs: SubmitUpdateParams::param_type(),
        outputs: SubmitUpdateOutput::param_type(),
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ConfirmUpdateParams {
    pub update_id: u64,
}

pub fn confirm_update() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "confirmUpdate",
        inputs: ConfirmUpdateParams::param_type(),
        outputs: Vec::new(),
    }
}

#[derive(Debug, Clone)]
pub struct ExecuteUpdateParams {
    pub update_id: u64,
    pub code: Option<Cell>,
}

pub fn execute_update() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "executeUpdate",
        inputs: ExecuteUpdateParams::param_type(),
        outputs: Vec::new(),
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

pub fn get_parameters() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "getParameters",
        inputs: Vec::new(),
        outputs: SetCodeMultisigParams::param_type(),
    }
}

#[derive(Debug, Clone)]
pub struct UpdateTransaction {
    pub id: u64,
    pub index: u8,
    pub signs: u8,
    pub confirmations_mask: u32,
    pub creator: HashBytes,
    pub new_code_hash: Option<HashBytes>,
    pub new_custodians: Option<Vec<HashBytes>>,
    pub new_req_confirms: Option<u8>,
    //#[abi(with = "updated_lifetime")]
    pub new_lifetime: Option<u32>,
}

//mod updated_lifetime {
//    use super::*;
//    use num_traits::cast::ToPrimitive;
//
//    pub fn unpack(value: &TokenValue) -> UnpackerResult<Option<u32>> {
//        let value = match value {
//            TokenValue::Optional(_, None) => return Ok(None),
//            TokenValue::Optional(_, Some(value)) => value,
//            _ => return Err(UnpackerError::InvalidAbi),
//        };
//
//        match value.as_ref() {
//            TokenValue::Uint(Uint { number, size: 32 }) => {
//                Ok(Some(number.to_u32().ok_or(UnpackerError::InvalidAbi)?))
//            }
//            TokenValue::Uint(Uint { number, size: 64 }) => {
//                let lifetime = number.to_u64().ok_or(UnpackerError::InvalidAbi)?;
//                Ok(Some(lifetime as u32))
//            }
//            _ => Err(UnpackerError::InvalidAbi),
//        }
//    }
//
//    pub fn param_type() -> ParamType {
//        Option::<u32>::param_type()
//    }
//}

pub mod v2_0 {
    use tycho_types::abi::NamedAbiType;

    use super::*;

    pub fn get_update_requests() -> &'static Function {
        declare_function! {
            abi: v2_3,
            header: [pubkey, time, expire],
            name: "getUpdateRequests",
            inputs: Vec::new(),
            outputs: {
                let mut param_types = UpdateTransaction::param_type();
                if let AbiType::Tuple(params) = &mut param_types {
                    if let Some(NamedAbiType {
                        ty: AbiType::Optional(param),
                        ..
                    }) = params.last_mut() {
                        if let AbiType::Uint(size) = param.as_mut() {
                            *size = 64;
                        }
                    }
                }

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
            inputs: Vec::new(),
            outputs: vec![
                AbiType::Array(Arc::new(UpdateTransaction::param_type())).named("updates")
            ],
        }
    }
}
