use std::sync::Arc;

use lru::LruCache;
use nekoton::transport::models::ExistingContract;
use nekoton_utils::TrustMe;
use parking_lot::Mutex;
use ton_block::MsgAddressInt;

use crate::sqlx_client::*;

#[derive(Clone)]
pub struct StatesCache {
    cache: Arc<Mutex<LruCache<MsgAddressInt, ExistingContract>>>,
    db: SqlxClient,
}

impl StatesCache {
    pub async fn new(sqlx_client: SqlxClient) -> Result<Self, anyhow::Error> {
        let states = sqlx_client.get_token_whitelist().await?;
        let mut res = LruCache::new(100);

        states.into_iter().for_each(|x| {
            if let Some(state) = x.state {
                res.put(nekoton_utils::repack_address(&x.address).trust_me(), {
                    let state: ExistingContract = serde_json::from_value(state).trust_me();
                    state
                });
            }
        });
        Ok(Self {
            cache: Arc::new(Mutex::new(res)),
            db: sqlx_client,
        })
    }

    pub async fn get(&self, address: &MsgAddressInt) -> Option<ExistingContract> {
        let state = {
            let mut lock = self.cache.lock();
            lock.get(address).cloned()
        };
        match state {
            Some(a) => Some(a),
            None => {
                let got = self.db.get_root_token(&address.to_string()).await.ok()?;
                match got.state {
                    None => None,
                    Some(state) => {
                        let state: Option<ExistingContract> = serde_json::from_value(state).ok();
                        state
                    }
                }
            }
        }
    }

    pub async fn insert(&self, key: MsgAddressInt, value: ExistingContract) {
        {
            self.cache.lock().put(key.clone(), value.clone());
        }

        if let Err(e) = self
            .db
            .update_root_token_state(&key.to_string(), serde_json::json!(value))
            .await
        {
            log::error!("Failed inserting root token state: {}", e)
        }
    }
}
