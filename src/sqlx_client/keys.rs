use crate::models::*;
use crate::prelude::*;
use crate::sqlx_client::*;

impl SqlxClient {
    pub async fn get_key(&self, api_key: &str) -> Result<Key, ServiceError> {
        sqlx::query_as!(
            Key,
            r#"SELECT id,
                    service_id as "service_id: _",
                    key,
                    secret,
                    whitelist,
                    created_at
                    FROM api_service_key WHERE key = $1"#,
            &api_key
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }
    pub async fn get_key_by_service_id(&self, service_id: &ServiceId) -> Result<Key, ServiceError> {
        sqlx::query_as!(
            Key,
            r#"SELECT id,
                    service_id as "service_id: _",
                    key,
                    secret,
                    whitelist,
                    created_at
                    FROM api_service_key WHERE service_id = $1"#,
            service_id: ServiceId,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }
}
