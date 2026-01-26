use nekoton_abi::*;
use ton_abi::{Param, ParamType};

use crate::utils::declare_function;

pub fn constructor() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_0,
        header: [pubkey, time, expire],
        name: "constructor",
        inputs: vec![
            Param::new("owners", ParamType::Array(Box::new(ParamType::Uint(256)))),
            Param::new("reqConfirms", ParamType::Uint(8)),
        ],
        outputs: Vec::new(),
    }
}

pub fn send_transaction() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_0,
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
        abi: v2_0,
        header: [pubkey, time, expire],
        name: "submitTransaction",
        inputs: vec![
            Param::new("dest", ParamType::Address),
            Param::new("value", ParamType::Uint(128)),
            Param::new("bounce", ParamType::Bool),
            Param::new("allBalance", ParamType::Bool),
            Param::new("payload", ParamType::Cell)
        ],
        outputs: vec![Param::new("transId", ParamType::Uint(64))],
    }
}

pub fn confirm_transaction() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_0,
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
}

pub fn get_transactions() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_0,
        header: [pubkey, time, expire],
        name: "getTransactions",
        inputs: Vec::new(),
        outputs: vec![
            Param::new("transactions", ParamType::Array(Box::new(MultisigTransaction::param_type())))
        ]
    }
}

#[derive(Debug, UnpackAbi, KnownParamType, Copy, Clone)]
pub struct MultisigCustodian {
    #[abi(uint8)]
    pub index: u8,
    #[abi(uint256)]
    pub pubkey: ton_types::UInt256,
}

pub fn get_custodians() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_0,
        header: [pubkey, time, expire],
        name: "getCustodians",
        inputs: Vec::new(),
        outputs: vec![
            Param::new("custodians", ParamType::Array(Box::new(MultisigCustodian::param_type())))
        ]
    }
}

pub mod safe_multisig {
    use super::*;

    #[derive(Debug, Clone, Copy, UnpackAbiPlain, KnownParamTypePlain)]
    pub struct SafeMultisigParams {
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
    }

    pub fn get_parameters() -> &'static ton_abi::Function {
        declare_function! {
            abi: v2_0,
            header: [pubkey, time, expire],
            name: "getParameters",
            inputs: Vec::new(),
            outputs: SafeMultisigParams::param_type(),
        }
    }
}

pub mod set_code_multisig {
    use super::*;

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
            abi: v2_0,
            header: [pubkey, time, expire],
            name: "getParameters",
            inputs: Vec::new(),
            outputs: SetCodeMultisigParams::param_type(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_function_ids() {
        assert_eq!(constructor().input_id, 0x6c1e693c);
        assert_eq!(send_transaction().input_id, 0x4cee646c);
        assert_eq!(submit_transaction().input_id, 0x131d82cd);
        assert_eq!(confirm_transaction().input_id, 0x1aa740ed);
        assert_eq!(safe_multisig::get_parameters().input_id, 0x6d28dde8);
        assert_eq!(set_code_multisig::get_parameters().input_id, 0x66b8710c);
        assert_eq!(get_transactions().input_id, 0x73122f72);
        assert_eq!(get_custodians().input_id, 0x5b00d859);
    }
}
