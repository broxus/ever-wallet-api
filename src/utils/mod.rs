use std::hash::BuildHasherDefault;

use anyhow::Result;
use rustc_hash::FxHasher;
use ton_block::Deserializable;
use tycho_types::boc::Boc;
use tycho_types::cell::CellBuilder;
use tycho_types::models::Transaction;

pub use self::encoding::*;
pub use self::existing_contract::*;
pub use self::pending_messages_queue::*;
pub use self::shard_utils::*;
pub use self::token_wallet::*;
pub use self::tx_context::*;

mod encoding;
mod existing_contract;
mod pending_messages_queue;
mod shard_utils;
mod token_wallet;
mod tx_context;
mod ton_wallet;
mod wallets;

pub type FxDashMap<K, V> = dashmap::DashMap<K, V, BuildHasherDefault<FxHasher>>;
pub type FxDashSet<K> = dashmap::DashSet<K, BuildHasherDefault<FxHasher>>;

pub fn conver_to_old_transaction(transaction: &Transaction) -> Result<ton_block::Transaction> {
    let cell = CellBuilder::build_from(transaction)?;
    let bytes = Boc::encode(cell);
    let cell = ton_types::deserialize_tree_of_cells(&mut &*bytes)?;
    ton_block::Transaction::construct_from_cell(cell)
}
