use std::str::FromStr;
use std::sync::Arc;

use lru::LruCache;
use nekoton::core::models::TokenWalletVersion;
use nekoton_utils::TrustMe;
use parking_lot::Mutex;
use ton_block::MsgAddressInt;

use crate::models::sqlx::*;
use crate::sqlx_client::*;

#[derive(Clone)]
/// Maps token wallet address to Owner info
pub struct OwnersCache {
    cache: Arc<Mutex<LruCache<MsgAddressInt, OwnerInfo>>>,
    db: SqlxClient,
}

impl OwnersCache {
    pub async fn get(&self, address: &MsgAddressInt) -> Option<OwnerInfo> {
        let info = {
            let mut lock = self.cache.lock();
            lock.get(address).cloned()
        };
        let info = match info {
            Some(a) => a,
            None => {
                let got = self
                    .db
                    .get_token_owner_by_address(address.to_string())
                    .await
                    .ok()?;
                OwnerInfo {
                    owner_address: MsgAddressInt::from_str(&format!(
                        "{}:{}",
                        got.owner_account_workchain_id, got.owner_account_hex
                    ))
                    .trust_me(),
                    root_address: nekoton_utils::repack_address(&got.root_address).trust_me(),
                    code_hash: got.code_hash,
                    version: got.version.into(),
                }
            }
        };
        Some(info)
    }
    pub async fn insert(&self, key: MsgAddressInt, value: OwnerInfo) {
        {
            self.cache.lock().put(key.clone(), value.clone());
        }
        let owner = TokenOwnerFromDb {
            address: key.to_string(),
            owner_account_workchain_id: value.owner_address.workchain_id(),
            owner_account_hex: value.owner_address.address().to_hex_string(),
            root_address: value.root_address.to_string(),
            code_hash: value.code_hash,
            created_at: chrono::Utc::now().naive_utc(), //doesn't matter
            version: value.version.into(),
        };
        if let Err(e) = self.db.new_token_owner(&owner).await {
            log::error!("Failed inserting owner info: {}", e)
        }
    }
}

#[derive(Clone, Debug)]
pub struct OwnerInfo {
    pub owner_address: MsgAddressInt,
    pub root_address: MsgAddressInt,
    pub code_hash: Vec<u8>,
    pub version: TokenWalletVersion,
}

impl OwnersCache {
    pub async fn new(sqlx_client: SqlxClient) -> Result<Self, anyhow::Error> {
        let balances = sqlx_client.get_all_token_owners().await?;
        // no more than 10 mb
        let mut res = LruCache::new(5000);
        balances.into_iter().for_each(|x| {
            res.put(
                nekoton_utils::repack_address(&x.address).trust_me(),
                OwnerInfo {
                    owner_address: MsgAddressInt::from_str(&format!(
                        "{}:{}",
                        x.owner_account_workchain_id, x.owner_account_hex
                    ))
                    .trust_me(),
                    root_address: nekoton_utils::repack_address(&x.root_address).trust_me(),
                    code_hash: x.code_hash,
                    version: x.version.into(),
                },
            );
        });
        Ok(Self {
            cache: Arc::new(Mutex::new(res)),
            db: sqlx_client,
        })
    }
}
