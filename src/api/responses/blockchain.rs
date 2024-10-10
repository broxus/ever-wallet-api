use opg::OpgModel;
use serde::Serialize;

use crate::models::*;

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("BlockchainInfoResponse")]
pub struct BlockchainInfoResponse {
    pub tip_block_ts: u32,
    pub synced: bool,
    pub subscriber_pending_messages: usize,
    pub masterchain_height: u32,
    pub masterchain_last_updated: i64,
    pub network_id: i32,
}

impl From<BlockchainInfo> for BlockchainInfoResponse {
    fn from(r: BlockchainInfo) -> Self {
        Self {
            tip_block_ts: r.tip_block_ts,
            synced: r.synced,
            subscriber_pending_messages: r.subscriber_pending_messages,
            masterchain_height: r.masterchain_height,
            masterchain_last_updated: r.masterchain_last_updated,
            network_id: r.network_id,
        }
    }
}
