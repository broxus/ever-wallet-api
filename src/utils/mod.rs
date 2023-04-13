use std::hash::BuildHasherDefault;

use rustc_hash::FxHasher;

pub use self::address::*;
pub use self::encoding::*;
pub use self::existing_contract::*;
pub use self::pending_messages_queue::*;
pub use self::shard_utils::*;
pub use self::token_wallet::*;
pub use self::tx_context::*;

mod address;
mod encoding;
mod existing_contract;
mod pending_messages_queue;
mod shard_utils;
mod token_wallet;
mod tx_context;

pub type FxDashMap<K, V> = dashmap::DashMap<K, V, BuildHasherDefault<FxHasher>>;
pub type FxDashSet<K> = dashmap::DashSet<K, BuildHasherDefault<FxHasher>>;
