mod requests;

pub use self::requests::*;

use async_trait::async_trait;
use http::Method;

use crate::prelude::ServiceError;

#[async_trait]
pub trait CallbackClient: Send + Sync {
    async fn send(&self, url: String, payload: AccountTransactionEvent)
        -> Result<(), ServiceError>;
}

#[derive(Clone)]
pub struct CallbackClientImpl {
    client: reqwest::Client,
}

impl CallbackClientImpl {
    pub fn new() -> Self {
        Self {
            client: reqwest::ClientBuilder::new().build().unwrap(),
        }
    }
}

#[async_trait]
impl CallbackClient for CallbackClientImpl {
    async fn send(
        &self,
        url: String,
        payload: AccountTransactionEvent,
    ) -> Result<(), ServiceError> {
        let res = self
            .client
            .request(Method::POST, &url)
            .json(&payload)
            .send()
            .await
            .map_err(ServiceError::from)?;

        if res.status() != http::StatusCode::OK {
            Err(ServiceError::Other(anyhow::Error::msg(format!(
                "Received status is not 200. Payload: {:#?}. Receive: {:?}.",
                payload, res
            ))))
        } else {
            Ok(())
        }
    }
}
