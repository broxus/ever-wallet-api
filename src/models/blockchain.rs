use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainInfo {
    pub tip_block_ts: u32,
    pub synced: bool,
    pub subscriber_pending_messages: usize,
    pub masterchain_height: u32,
    pub masterchain_last_updated: i64,
    pub network_id: i32,
}
