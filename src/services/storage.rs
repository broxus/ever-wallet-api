use tycho_types::{
    abi::UnsignedExternalMessage,
    cell::{CellBuilder, HashBytes},
};
use tycho_util::time::now_sec;

use crate::utils::FxDashMap;

#[derive(Default)]
pub struct StorageHandler {
    message_collection: FxDashMap<HashBytes, UnsignedExternalMessage>,
}

impl StorageHandler {
    pub fn add_message(&self, message: UnsignedExternalMessage) -> HashBytes {
        let cell_builder =
            CellBuilder::build_from(message.clone().without_signature().unwrap()).unwrap();
        let message_hash = *cell_builder.repr_hash();
        self.message_collection.insert(message_hash, message);
        message_hash
    }

    pub fn get_message(&self, hash: &HashBytes) -> Option<UnsignedExternalMessage> {
        let now = now_sec();
        self.message_collection
            .retain(|_, message| message.expire_at() > now);
        let message = self.message_collection.get(hash).map(|x| x.value().clone());
        message
    }
}
