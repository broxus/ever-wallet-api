use tycho_types::{abi::UnsignedExternalMessage, cell::CellBuilder};
use tycho_util::time::now_sec;

use crate::utils::FxDashMap;

#[derive(Default)]
pub struct StorageHandler {
    message_collection: FxDashMap<String, UnsignedExternalMessage>,
}

impl StorageHandler {
    pub fn add_message(&self, message: UnsignedExternalMessage) -> String {
        let cell_builder = CellBuilder::build_from(message.without_signature().unwrap()).unwrap();
        let message_hash = *cell_builder.repr_hash();
        let key = message_hash.to_string();
        self.message_collection.insert(key.clone(), message);
        key
    }

    pub fn get_message(&self, hash: &str) -> Option<UnsignedExternalMessage> {
        let now = now_sec();
        self.message_collection
            .retain(|_, message| message.expire_at() > now);
        let message = self.message_collection.get(hash).map(|x| x.value());
        message
    }
}
