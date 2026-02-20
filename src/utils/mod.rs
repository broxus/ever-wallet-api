use std::hash::BuildHasherDefault;

use rustc_hash::FxHasher;
use tycho_types::abi::AbiType;
use tycho_types::abi::AbiValue;
use tycho_types::abi::FromAbi;
use tycho_types::abi::IntoAbi;
use tycho_types::abi::NamedAbiType;
use tycho_types::abi::NamedAbiValue;
use tycho_types::abi::WithAbiType;

pub use self::encoding::*;
pub use self::existing_contract::*;
pub use self::pending_messages_queue::*;
pub use self::shard_utils::*;
pub use self::token_wallet::*;
pub use self::tx_context::*;

mod encoding;
mod existing_contract;
pub mod mnemonic;
pub mod multisig;
mod pending_messages_queue;
mod shard_utils;
mod token_wallet;
pub mod token_wallets;
pub mod ton_wallet;
mod tx_context;
mod wallets;

pub type FxDashMap<K, V> = dashmap::DashMap<K, V, BuildHasherDefault<FxHasher>>;
pub type FxDashSet<K> = dashmap::DashSet<K, BuildHasherDefault<FxHasher>>;

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
            let builder = tycho_types::abi::Function::builder($crate::utils::declare_function!(@abi_version $($abi)?), ($name).to_string())
                .with_headers($crate::utils::declare_function!(@header $($($header),+)?))
                .with_inputs($inputs as Vec<tycho_types::abi::NamedAbiType>)
                .with_outputs($outputs as Vec<tycho_types::abi::NamedAbiType>);

            $crate::utils::declare_function!(@function_id builder $($id)?);

            builder
                .build()
        })
    };

    (@function_id $builder:ident $id:literal) => { let $builder = $builder.with_id($id); };
    (@function_id $builder:ident ) => {};

    (@abi_version) => { tycho_types::abi::AbiVersion::V2_2 };
    (@abi_version v2_0) => { tycho_types::abi::AbiVersion::V2_0 };
    (@abi_version v2_1) => { tycho_types::abi::AbiVersion::V2_1 };
    (@abi_version v2_2) => { tycho_types::abi::AbiVersion::V2_2 };
    (@abi_version v2_3) => { tycho_types::abi::AbiVersion::V2_3 };
    (@abi_version v2_7) => { tycho_types::abi::AbiVersion::V2_7 };

    (@header) => { Vec::new() };
    (@header $($header:ident),+) => {
        vec![$($crate::utils::declare_function!(@header_item $header)),+]
    };
    (@header_item pubkey) => {
        tycho_types::abi::AbiHeaderType::PublicKey
    };
    (@header_item time) => {
        tycho_types::abi::AbiHeaderType::Time
    };
    (@header_item expire) => {
        tycho_types::abi::AbiHeaderType::Expire
    };
}

pub(crate) use declare_function;

pub mod serde_string {
    use std::str::FromStr;

    use serde::de::Error;
    use serde::{Deserialize, Serialize};

    pub fn serialize<S>(data: &dyn std::fmt::Display, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        data.to_string().serialize(serializer)
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: FromStr,
        T::Err: std::fmt::Display,
    {
        String::deserialize(deserializer)
            .and_then(|data| T::from_str(&data).map_err(D::Error::custom))
    }
}

pub mod serde_address {
    use serde::de::Error;
    use serde::Deserialize;
    use tycho_types::models::{StdAddr, StdAddrFormat};

    pub fn serialize<S>(data: &StdAddr, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&data.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<StdAddr, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = String::deserialize(deserializer)?;

        let (address, _) = StdAddr::from_str_ext(&data, StdAddrFormat::any())
            .map_err(|_| D::Error::custom("Invalid address"))?;
        Ok(address)
    }
}

pub mod serde_optional_string {
    use std::fmt;
    use std::str::FromStr;

    use serde::de::Error;
    use serde::{Deserialize, Serialize};

    pub fn serialize<S, T>(data: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        T: fmt::Display,
    {
        data.as_ref().map(ToString::to_string).serialize(serializer)
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: FromStr,
        T::Err: fmt::Display,
    {
        Option::<String>::deserialize(deserializer).and_then(|data| {
            data.map(|data| T::from_str(&data).map_err(Error::custom))
                .transpose()
        })
    }
}

pub mod serde_cell {
    use serde::de::Error;
    use serde::Deserialize;
    use tycho_types::boc::Boc;
    use tycho_types::cell::Cell;

    pub fn serialize<S>(data: &Cell, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let bytes = Boc::encode_base64(data);
        serializer.serialize_str(&bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Cell, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = String::deserialize(deserializer)?;
        let cell = Boc::decode_base64(&data).map_err(|_| D::Error::custom("Invalid cell"))?;
        Ok(cell)
    }
}

pub struct InputMessage(pub Vec<NamedAbiValue>);

pub struct ContractCall {
    pub inputs: Vec<NamedAbiValue>,
    pub outputs: Vec<NamedAbiValue>,
}

pub trait IntoAbiPlain: IntoAbi {
    fn into_abi_plain(self) -> Vec<NamedAbiValue>
    where
        Self: Sized,
    {
        let tuple = self.into_abi();
        match tuple {
            AbiValue::Tuple(tuple) => tuple,
            _ => vec![],
        }
    }
    fn as_abi_plain(&self) -> Vec<NamedAbiValue> {
        let tuple = self.as_abi();
        match tuple {
            AbiValue::Tuple(tuple) => tuple,
            _ => vec![],
        }
    }
}

pub trait FromAbiPlain: FromAbi {
    fn from_abi_plain(value: Vec<NamedAbiValue>) -> anyhow::Result<Self> {
        Self::from_abi(AbiValue::Tuple(value))
    }
}

pub trait WithAbiTypePlain: WithAbiType {
    fn abi_type_plain() -> Vec<NamedAbiType> {
        match Self::abi_type() {
            AbiType::Tuple(tuple) => tuple.iter().cloned().collect(),
            _ => vec![],
        }
    }
}
