use tycho_types::abi::{AbiType, Function};

use crate::utils::declare_function;

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

macro_rules! declare_send_transaction_raw {
    ($($name:ident => [$($inputs:tt)*]),*,) => {
        $(pub fn $name() -> &'static Function {
            declare_function! {
                abi: v2_3,
                function_id: 0x169e3e11,
                header: [pubkey, time, expire],
                name: "sendTransactionRaw",
                inputs: declare_send_transaction_raw!(@inputs [$($inputs)*] []),
                outputs: Vec::new(),
            }
        })*
    };

    (@inputs [] [$($inputs:tt)*]) => {
        vec![$($inputs)*]
    };
    (@inputs [$(,)? _ $($rest:tt)*] [$($inputs:tt)*]) => {
        declare_send_transaction_raw!(@inputs [$($rest)*] [
            $($inputs)*
            tycho_types::abi::AbiType::Uint(8).named("flags"),
            tycho_types::abi::AbiType::Cell.named("message"),
        ])
    };
}

declare_send_transaction_raw! {
    send_transaction_raw_0 => [],
    send_transaction_raw_1 => [_],
    send_transaction_raw_2 => [_, _],
    send_transaction_raw_3 => [_, _, _],
    send_transaction_raw_4 => [_, _, _, _],
}
