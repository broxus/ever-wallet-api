use nekoton_abi::*;
use ton_abi::{Param, ParamType};

use crate::utils::declare_function;

pub fn constructor() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "constructor",
        inputs: vec![
            Param::new("owners", ParamType::Array(Box::new(ParamType::Uint(256)))),
            Param::new("reqConfirms", ParamType::Uint(8)),
            Param::new("lifetime", ParamType::Uint(32)),
        ],
        outputs: Vec::new(),
    }
}

pub fn send_transaction() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "sendTransaction",
        inputs: vec![
            Param::new("dest", ParamType::Address),
            Param::new("value", ParamType::Uint(128)),
            Param::new("bounce", ParamType::Bool),
            Param::new("flags", ParamType::Uint(8)),
            Param::new("payload", ParamType::Cell),
        ],
        outputs: Vec::new(),
    }
}

pub fn submit_transaction() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "submitTransaction",
        inputs: vec![
            Param::new("dest", ParamType::Address),
            Param::new("value", ParamType::Uint(128)),
            Param::new("bounce", ParamType::Bool),
            Param::new("allBalance", ParamType::Bool),
            Param::new("payload", ParamType::Cell),
            Param::new("stateInit", ParamType::Optional(Box::new(ParamType::Cell))),
        ],
        outputs: vec![Param::new("transId", ParamType::Uint(64))],
    }
}

pub fn confirm_transaction() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "confirmTransaction",
        inputs: vec![Param::new("transactionId", ParamType::Uint(64))],
        outputs: Vec::new(),
    }
}

#[derive(Debug, UnpackAbi, KnownParamType)]
pub struct MultisigTransaction {
    #[abi(uint64)]
    pub id: u64,
    #[abi(uint32, name = "confirmationsMask")]
    pub confirmation_mask: u32,
    #[abi(uint8, name = "signsRequired")]
    pub signs_required: u8,
    #[abi(uint8, name = "signsReceived")]
    pub signs_received: u8,
    #[abi(uint256)]
    pub creator: ton_types::UInt256,
    #[abi(uint8)]
    pub index: u8,
    #[abi(address)]
    pub dest: ton_block::MsgAddressInt,
    #[abi(uint128)]
    pub value: u128,
    #[abi(uint16, name = "sendFlags")]
    pub send_flags: u16,
    #[abi(cell)]
    pub payload: ton_types::Cell,
    #[abi(bool)]
    pub bounce: bool,
    #[abi]
    pub state_init: Option<ton_types::Cell>,
}

pub fn get_transactions() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "getTransactions",
        inputs: Vec::new(),
        outputs: vec![
            Param::new("transactions", ParamType::Array(Box::new(MultisigTransaction::param_type())))
        ]
    }
}

#[derive(Debug, Clone, Copy, UnpackAbi, KnownParamType)]
pub struct MultisigCustodian {
    #[abi(uint8)]
    pub index: u8,
    #[abi(uint256)]
    pub pubkey: ton_types::UInt256,
}

pub fn get_custodians() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "getCustodians",
        inputs: Vec::new(),
        outputs: vec![
            Param::new("custodians", ParamType::Array(Box::new(MultisigCustodian::param_type())))
        ]
    }
}

#[derive(Debug, Clone, UnpackAbiPlain, PackAbiPlain, KnownParamTypePlain)]
pub struct SubmitUpdateParams {
    #[abi]
    pub code_hash: Option<ton_types::UInt256>,
    #[abi]
    pub owners: Option<Vec<ton_types::UInt256>>,
    #[abi]
    pub req_confirms: Option<u8>,
    #[abi]
    pub lifetime: Option<u64>,
}

#[derive(Debug, Copy, Clone, UnpackAbiPlain, KnownParamTypePlain)]
pub struct SubmitUpdateOutput {
    #[abi(uint64)]
    pub update_id: u64,
}

pub fn submit_update() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "submitUpdate",
        inputs: SubmitUpdateParams::param_type(),
        outputs: SubmitUpdateOutput::param_type(),
    }
}

#[derive(Debug, Copy, Clone, UnpackAbiPlain, PackAbiPlain, KnownParamTypePlain)]
pub struct ConfirmUpdateParams {
    #[abi(uint64)]
    pub update_id: u64,
}

pub fn confirm_update() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "confirmUpdate",
        inputs: ConfirmUpdateParams::param_type(),
        outputs: Vec::new(),
    }
}

#[derive(Debug, Clone, UnpackAbiPlain, PackAbiPlain, KnownParamTypePlain)]
pub struct ExecuteUpdateParams {
    #[abi(uint64)]
    pub update_id: u64,
    #[abi]
    pub code: Option<ton_types::Cell>,
}

pub fn execute_update() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "executeUpdate",
        inputs: ExecuteUpdateParams::param_type(),
        outputs: Vec::new(),
    }
}

#[derive(Debug, Clone, Copy, UnpackAbiPlain, KnownParamTypePlain)]
pub struct SetCodeMultisigParams {
    #[abi(uint8, name = "maxQueuedTransactions")]
    pub max_queued_transactions: u8,
    #[abi(uint8, name = "maxCustodianCount")]
    pub max_custodian_count: u8,
    #[abi(uint64, name = "expirationTime")]
    pub expiration_time: u64,
    #[abi(uint128, name = "minValue")]
    pub min_value: u128,
    #[abi(uint8, name = "requiredTxnConfirms")]
    pub required_txn_confirms: u8,
    #[abi(uint8, name = "requiredUpdConfirms")]
    pub required_upd_confirms: u8,
}

pub fn get_parameters() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "getParameters",
        inputs: Vec::new(),
        outputs: SetCodeMultisigParams::param_type(),
    }
}

#[derive(Debug, Clone, UnpackAbi, KnownParamType)]
pub struct UpdateTransaction {
    #[abi(uint64)]
    pub id: u64,
    #[abi(uint8)]
    pub index: u8,
    #[abi(uint8)]
    pub signs: u8,
    #[abi(uint32)]
    pub confirmations_mask: u32,
    #[abi(uint256)]
    pub creator: ton_types::UInt256,
    #[abi]
    pub new_code_hash: Option<ton_types::UInt256>,
    #[abi]
    pub new_custodians: Option<Vec<ton_types::UInt256>>,
    #[abi]
    pub new_req_confirms: Option<u8>,
    #[abi(with = "updated_lifetime")]
    pub new_lifetime: Option<u32>,
}

mod updated_lifetime {
    use super::*;
    use num_traits::cast::ToPrimitive;

    pub fn unpack(value: &ton_abi::TokenValue) -> UnpackerResult<Option<u32>> {
        let value = match value {
            ton_abi::TokenValue::Optional(_, None) => return Ok(None),
            ton_abi::TokenValue::Optional(_, Some(value)) => value,
            _ => return Err(UnpackerError::InvalidAbi),
        };

        match value.as_ref() {
            ton_abi::TokenValue::Uint(ton_abi::Uint { number, size: 32 }) => {
                Ok(Some(number.to_u32().ok_or(UnpackerError::InvalidAbi)?))
            }
            ton_abi::TokenValue::Uint(ton_abi::Uint { number, size: 64 }) => {
                let lifetime = number.to_u64().ok_or(UnpackerError::InvalidAbi)?;
                Ok(Some(lifetime as u32))
            }
            _ => Err(UnpackerError::InvalidAbi),
        }
    }

    pub fn param_type() -> ParamType {
        Option::<u32>::param_type()
    }
}

pub mod v2_0 {
    use super::*;

    pub fn get_update_requests() -> &'static ton_abi::Function {
        declare_function! {
            abi: v2_3,
            header: [pubkey, time, expire],
            name: "getUpdateRequests",
            inputs: Vec::new(),
            outputs: {
                let mut param_types = UpdateTransaction::param_type();
                if let ton_abi::ParamType::Tuple(params) = &mut param_types {
                    if let Some(ton_abi::Param {
                        kind: ton_abi::ParamType::Optional(param),
                        ..
                    }) = params.last_mut() {
                        if let ton_abi::ParamType::Uint(size) = param.as_mut() {
                            *size = 64;
                        }
                    }
                }

                vec![Param::new("updates", ParamType::Array(Box::new(param_types)))]
            },
        }
    }
}

pub mod v2_1 {
    use super::*;

    pub fn get_update_requests() -> &'static ton_abi::Function {
        declare_function! {
            abi: v2_3,
            header: [pubkey, time, expire],
            name: "getUpdateRequests",
            inputs: Vec::new(),
            outputs: vec![
                Param::new("updates", ParamType::Array(Box::new(UpdateTransaction::param_type())))
            ],
        }
    }
}
