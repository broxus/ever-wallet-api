use std::sync::Arc;

use tycho_types::{
    abi::{AbiType, FromAbi, Function, IntoAbi, NamedAbiType, WithAbiType},
    cell::{Cell, HashBytes},
    models::StdAddr,
};

use crate::utils::declare_function;

pub fn constructor() -> &'static Function {
    declare_function! {
        abi: v2_0,
        header: [pubkey, time, expire],
        name: "constructor",
        inputs: vec![
            AbiType::Array(Arc::new(AbiType::Uint(256))).named("owners"),
            AbiType::Uint(8).named("reqConfirms"),
        ],
        outputs: vec![] as Vec<NamedAbiType>,
    }
}

pub fn send_transaction() -> &'static Function {
    declare_function! {
        abi: v2_0,
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
        abi: v2_0,
        header: [pubkey, time, expire],
        name: "submitTransaction",
        inputs: vec![
            AbiType::Address.named("dest"),
            AbiType::Uint(128).named("value"),
            AbiType::Bool.named("bounce"),
            AbiType::Bool.named("allBalance"),
            AbiType::Cell.named("payload"),
        ],
        outputs: vec![
            AbiType::Uint(64).named("transId")
            ],
    }
}

pub fn confirm_transaction() -> &'static Function {
    declare_function! {
        abi: v2_0,
        header: [pubkey, time, expire],
        name: "confirmTransaction",
        inputs: vec![
            AbiType::Uint(64).named("transactionId"),
            ],
        outputs: vec![] as Vec<NamedAbiType>,
    }
}

#[derive(IntoAbi, FromAbi, WithAbiType, Eq, PartialEq, Debug, Clone)]
pub struct MultisigTransaction {
    pub id: u64,
    #[abi(name = "confirmationsMask")]
    pub confirmation_mask: u32,
    #[abi(name = "signsRequired")]
    pub signs_required: u8,
    #[abi(name = "signsReceived")]
    pub signs_received: u8,
    pub creator: HashBytes,
    pub index: u8,
    pub dest: StdAddr,
    pub value: u128,
    #[abi(name = "sendFlags")]
    pub send_flags: u16,
    pub payload: Cell,
    pub bounce: bool,
}

pub fn get_transactions() -> &'static Function {
    declare_function! {
        abi: v2_0,
        header: [pubkey, time, expire],
        name: "getTransactions",
        inputs: vec![] as Vec<NamedAbiType>,
        outputs: vec![
            AbiType::Array(Arc::new(MultisigTransaction::abi_type())).named("transactions")
        ]
    }
}

#[derive(IntoAbi, FromAbi, WithAbiType, Eq, PartialEq, Debug, Clone)]
pub struct MultisigCustodian {
    pub index: u8,
    pub pubkey: HashBytes,
}

pub fn get_custodians() -> &'static Function {
    declare_function! {
        abi: v2_0,
        header: [pubkey, time, expire],
        name: "getCustodians",
        inputs: vec![] as Vec<NamedAbiType>,
        outputs: vec![
            AbiType::Array(Arc::new(MultisigCustodian::abi_type())).named("custodians")

        ]
    }
}

pub mod safe_multisig {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    pub struct SafeMultisigParams {
        pub max_queued_transactions: u8,
        pub max_custodian_count: u8,
        pub expiration_time: u64,
        pub min_value: u128,
        pub required_txn_confirms: u8,
    }

    impl SafeMultisigParams {
        fn abi_type() -> Vec<NamedAbiType> {
            vec![
                AbiType::Uint(8).named("maxQueuedTransactions"),
                AbiType::Uint(8).named("maxCustodianCount"),
                AbiType::Uint(64).named("expirationTime"),
                AbiType::Uint(128).named("minValue"),
                AbiType::Uint(8).named("requiredTxnConfirms"),
            ]
        }
    }

    pub fn get_parameters() -> &'static Function {
        declare_function! {
            abi: v2_0,
            header: [pubkey, time, expire],
            name: "getParameters",
            inputs: vec![] as Vec<NamedAbiType>,
            outputs: SafeMultisigParams::abi_type(),
        }
    }
}

pub mod set_code_multisig {
    use super::*;

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
            abi: v2_0,
            header: [pubkey, time, expire],
            name: "getParameters",
            inputs: vec![] as Vec<NamedAbiType>,
            outputs: SetCodeMultisigParams::abi_type(),
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
