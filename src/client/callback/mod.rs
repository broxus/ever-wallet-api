use anyhow::Result;
use chrono::Utc;
use http::Method;
use nekoton_utils::TrustMe;
use reqwest::Url;

use crate::models::*;

#[derive(Clone)]
pub struct CallbackClient {
    client: reqwest::Client,
}

impl CallbackClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::ClientBuilder::new().build().trust_me(),
        }
    }
}

impl Default for CallbackClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CallbackClient {
    pub async fn send(
        &self,
        url: String,
        payload: AccountTransactionEvent,
        secret: String,
    ) -> Result<()> {
        let nonce = Utc::now().naive_utc().timestamp() * 1000;

        let body = serde_json::to_string(&payload)?;

        let full_url = Url::parse(&url)?;

        let sign = calc_sign(body, full_url.path().to_string(), nonce, secret);

        let res = self
            .client
            .request(Method::POST, &url)
            .header("SIGN", sign)
            .header("TIMESTAMP", nonce.to_string())
            .json(&payload)
            .send()
            .await?;

        if res.status() != http::StatusCode::OK {
            anyhow::bail!(format!(
                "Received status is not 200. Payload: {:#?}. Receive: {:?}.",
                payload, res
            ))
        }

        Ok(())
    }
}

fn calc_sign(body: String, url: String, timestamp_ms: i64, secret: String) -> String {
    let concat = format!("{}{}{}", timestamp_ms, url, body);
    let calculated_signature = hmac_sha256::HMAC::mac(concat.as_bytes(), secret.as_bytes());
    base64::encode(&calculated_signature)
}
