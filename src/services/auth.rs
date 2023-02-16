use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use chrono::{NaiveDateTime, Utc};
use parking_lot::Mutex;

use crate::models::*;
use crate::sqlx_client::*;

pub const TIMESTAMP_EXPIRED_SEC: i64 = 10;

#[derive(Clone)]
pub struct AuthService {
    sqlx_client: SqlxClient,
    keys_hash: Arc<Mutex<HashMap<String, Key>>>,
}

impl AuthService {
    pub fn new(sqlx_client: SqlxClient) -> Self {
        Self {
            sqlx_client,
            keys_hash: Default::default(),
        }
    }

    pub async fn authenticate(
        &self,
        api_key: &str,
        timestamp: &str,
        signature: &str,
        path: &str,
        body: &str,
        real_ip: Option<String>,
    ) -> anyhow::Result<ServiceId> {
        let key = self
            .get_key(api_key)
            .await
            .map_err(|_| anyhow::Error::msg(format!("Can not find api key {} in db", api_key)))?;

        if let Some(whitelist) = key.whitelist {
            let whitelist: Vec<String> = serde_json::from_value(whitelist)
                .map_err(|_| anyhow::Error::msg("Can not parse ips whitelist"))?;

            let real_ip =
                real_ip.ok_or_else(|| anyhow::Error::msg("Failed to read x-real-ip header"))?;

            if !whitelist.contains(&real_ip) {
                anyhow::bail!(format!("Ip {} is not in whitelist.", real_ip))
            }
        }

        let timestamp_ms = timestamp
            .parse::<i64>()
            .map_err(|_| anyhow::Error::msg("Failed to read timestamp header"))?;

        let timestamp = timestamp_ms / 1000;

        let now = Utc::now().naive_utc();
        let then = NaiveDateTime::from_timestamp_opt(timestamp, 0).context("Invalid timestamp")?;

        let delta = (now - then).num_seconds();
        if delta > TIMESTAMP_EXPIRED_SEC {
            anyhow::bail!(format!(
                "TIMESTAMP expired. server time: {}, header time: {}",
                now, then
            ))
        }

        let concat = format!("{}{}{}", timestamp_ms, path, body);

        let calculated_signature = hmac_sha256::HMAC::mac(concat.as_bytes(), key.secret.as_bytes());

        let expected_signature = base64::decode(signature)?;

        if calculated_signature != expected_signature.as_slice() {
            anyhow::bail!("Invalid signature");
        }

        Ok(key.service_id)
    }

    async fn get_key(&self, api_key: &str) -> anyhow::Result<Key> {
        let cached_key = {
            let lock = self.keys_hash.lock();
            lock.get(api_key).cloned()
        };

        if let Some(key) = cached_key {
            return Ok(key);
        }

        let key: Key = self.sqlx_client.get_key(api_key).await?;

        {
            let mut lock = self.keys_hash.lock();
            lock.insert(api_key.to_string(), key.clone());
        }

        Ok(key)
    }
}
