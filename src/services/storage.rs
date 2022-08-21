use std::time::SystemTime;

use everscale_network::utils::FxDashMap;
use nekoton::crypto::UnsignedMessage;
use nekoton_utils::TrustMe;

#[derive(Default)]
pub struct StorageHandler {
    message_collection: FxDashMap<String, Box<dyn UnsignedMessage>>,
}

impl StorageHandler {
    pub fn add_message(&self, message: Box<dyn UnsignedMessage>) -> String {
        let key = hex::encode(message.hash());
        self.message_collection.insert(key.clone(), message);
        key
    }

    pub fn get_message(&self, hash: &str) -> Option<Box<dyn UnsignedMessage>> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .trust_me()
            .as_secs() as u32;
        self.message_collection.retain(|_, v| v.expire_at() > now);
        let message = self.message_collection.get(hash).map(|x| x.value().clone());
        message
    }
}
