use crate::models::key::Key;
use crate::prelude::RedisPooledConnection;

pub trait KeysRepoCache {
    fn get(&mut self, api_key: &str) -> Result<Option<Key>, anyhow::Error>;
    fn set(&mut self, key: &Key) -> Result<(), anyhow::Error>;
}

#[derive(derive_more::Constructor)]
pub struct KeysRepoCacheImpl<'a> {
    redis_conn: &'a mut RedisPooledConnection,
}

impl<'a> KeysRepoCache for KeysRepoCacheImpl<'a> {
    fn get(&mut self, api_key: &str) -> Result<Option<Key>, anyhow::Error> {
        let key: Option<String> = redis::cmd("GET")
            .arg(create_key(api_key))
            .query(&mut **self.redis_conn)?;

        Ok(key.and_then(|key| serde_json::from_str(&key).ok()))
    }

    fn set(&mut self, key: &Key) -> Result<(), anyhow::Error> {
        let serialized = serde_json::to_string(key).unwrap();

        redis::cmd("SET")
            .arg(create_key(&key.key))
            .arg(serialized)
            .query(&mut **self.redis_conn)?;

        Ok(())
    }
}

#[inline]
fn create_key(api_key: &str) -> String {
    format!("ton-wallet-api-rs-key-{}", api_key)
}
