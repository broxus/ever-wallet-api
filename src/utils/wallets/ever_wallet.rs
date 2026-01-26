use tycho_types::abi::{AbiType, Function};

use crate::utils::wallets::ever_wallet::utils::declare_function;

pub mod utils {
    macro_rules! declare_function {
        (
            $(abi: $abi:ident,)?
            $(function_id: $id:literal,)?
            $(header: [$($header:ident),+],)?
            name: $name:literal,
            inputs: $inputs:expr,
            outputs: $outputs:expr$(,)?
        ) => {
            static ONCE: std::sync::OnceLock<tycho_types::abi::Function> = std::sync::OnceLock::new();
            ONCE.get_or_init(|| {
                let mut builder = tycho_types::abi::Function::builder($crate::utils::wallets::ever_wallet::utils::declare_function!(@abi_version $($abi)?), ($name).to_string())
                    .with_headers($crate::utils::wallets::ever_wallet::utils::declare_function!(@header $($($header),+)?))
                    .with_inputs($inputs)
                    .with_outputs($outputs);

                $crate::utils::wallets::ever_wallet::utils::declare_function!(@function_id builder $($id)?);

                builder
                    .build()
            })
        };

        (@function_id $builder:ident $id:literal) => { $builder.with_id($id) };
        (@function_id $builder:ident ) => {};

        (@abi_version) => { tycho_types::abi::AbiVersion::V2_2 };
        (@abi_version v2_0) => { tycho_types::abi::AbiVersion::V2_0 };
        (@abi_version v2_1) => { tycho_types::abi::AbiVersion::V2_1 };
        (@abi_version v2_2) => { tycho_types::abi::AbiVersion::V2_2 };
        (@abi_version v2_3) => { tycho_types::abi::AbiVersion::V2_3 };
        (@abi_version v2_7) => { tycho_types::abi::AbiVersion::V2_7 };

        (@header) => { Vec::new() };
        (@header $($header:ident),+) => {
            vec![$($crate::utils::wallets::ever_wallet::utils::declare_function!(@header_item $header)),+]
        };
        (@header_item pubkey) => {
            tycho_types::abi::AbiHeaderType::Pubkey
        };
        (@header_item time) => {
            tycho_types::abi::AbiHeaderType::Time
        };
        (@header_item expire) => {
            tycho_types::abi::AbiHeaderType::Expire
        };
    }

    pub(crate) use declare_function;
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
