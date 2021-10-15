use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use hmac::{Hmac, Mac, NewMac};
use http::{header::HeaderValue, HeaderMap};
use parking_lot::Mutex;
use sha2::Sha256;

use crate::models::*;
use crate::prelude::*;
use crate::sqlx_client::*;

type HmacSha256 = Hmac<Sha256>;

pub const TIMESTAMP_EXPIRED_SEC: i64 = 10;

#[async_trait]
pub trait AuthService: Send + Sync + 'static {
    async fn authenticate(
        &self,
        body: String,
        path: warp::path::FullPath,
        headers: HeaderMap<HeaderValue>,
    ) -> Result<ServiceId, ServiceError>;
}

#[derive(Clone)]
pub struct AuthServiceImpl {
    sqlx_client: SqlxClient,
    keys_hash: Arc<Mutex<HashMap<String, Key>>>,
}

impl AuthServiceImpl {
    pub fn new(sqlx_client: SqlxClient) -> Self {
        Self {
            sqlx_client,
            keys_hash: Default::default(),
        }
    }
    async fn get_key(&self, api_key: &str) -> Result<Key, ServiceError> {
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

#[async_trait]
impl AuthService for AuthServiceImpl {
    async fn authenticate(
        &self,
        body: String,
        path: warp::path::FullPath,
        headers: HeaderMap<HeaderValue>,
    ) -> Result<ServiceId, ServiceError> {
        let api_key = headers
            .get("api-key")
            .ok_or_else(|| ServiceError::Auth("API-KEY Header Not Found".to_string()))?
            .to_str()
            .map_err(|_| ServiceError::Auth("API-KEY Header Not Found".to_string()))?;

        let key = self
            .get_key(api_key)
            .await
            .map_err(|_| ServiceError::Auth(format!("Can not find api key {} in db", api_key)))?;

        if let Some(whitelist) = key.whitelist {
            let whitelist: Vec<String> = serde_json::from_value(whitelist)
                .map_err(|_| ServiceError::Auth("Can not parse ips whitelist".to_string()))?;

            if let Some(real_ip) = headers.get("x-real-ip") {
                let ip = real_ip
                    .to_str()
                    .map_err(|_| ServiceError::Auth("Can not parse ip header".to_string()))?;

                if !whitelist.contains(&ip.to_string()) {
                    return Err(ServiceError::Auth(format!(
                        "Ip {} is not in whitelist.",
                        ip
                    )));
                }
            } else {
                return Err(ServiceError::Auth("Can not parse request ip".to_string()));
            }
        }

        let timestamp_header = headers
            .get("timestamp")
            .ok_or_else(|| ServiceError::Auth("TIMESTAMP Header Not Found".to_string()))?;
        let timestamp_str = timestamp_header
            .to_str()
            .map_err(|_| ServiceError::Auth("TIMESTAMP Header Not Found".to_string()))?;
        let timestamp_ms = timestamp_str
            .parse::<i64>()
            .map_err(|_| ServiceError::Auth("TIMESTAMP Header Not Found".to_string()))?;

        let timestamp = timestamp_ms / 1000;

        let then = NaiveDateTime::from_timestamp(timestamp, 0);

        let now = Utc::now().naive_utc();
        let delta = (now - then).num_seconds();

        if delta > TIMESTAMP_EXPIRED_SEC {
            return Err(ServiceError::Auth(format!(
                "TIMESTAMP expired. server time: {}, header time: {}",
                now, then
            )));
        }

        let mut mac = HmacSha256::new_from_slice(key.secret.as_bytes())
            .map_err(|_| ServiceError::Auth("Secret is not hmac sha256".to_string()))?;

        let signing_phrase = format!("{}{}{}", timestamp_ms, path.as_str(), body);
        mac.update(signing_phrase.as_bytes());

        let sign_header = headers
            .get("sign")
            .ok_or_else(|| ServiceError::Auth("SIGN Header Not Found".to_string()))?;
        let sign_str = sign_header
            .to_str()
            .map_err(|_| ServiceError::Auth("SIGN Header Not Found".to_string()))?;
        let sign = base64::decode(sign_str)
            .map_err(|_| ServiceError::Auth("SIGN Header Not Found".to_string()))?;

        mac.verify(&sign)
            .map_err(|_| ServiceError::Auth("SIGN invalid Not Found".to_string()))?;

        Ok(key.service_id)
    }
}
