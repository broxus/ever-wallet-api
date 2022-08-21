use anyhow::Result;

use crate::models::*;
use crate::sqlx_client::*;

impl SqlxClient {
    pub async fn create_api_service(
        &self,
        service_id: ServiceId,
        service_name: &str,
    ) -> Result<ApiServiceDb> {
        sqlx::query_as!(
            ApiServiceDb,
            r#"INSERT INTO api_service
                (id, name)
                VALUES ($1, $2)
                RETURNING
                id as "id: _", name, created_at"#,
            service_id as ServiceId,
            service_name
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }

    pub async fn create_api_service_key(
        &self,
        service_id: ServiceId,
        key: &str,
        secret: &str,
    ) -> Result<ApiServiceKeyDb> {
        sqlx::query_as!(
            ApiServiceKeyDb,
            r#"INSERT INTO api_service_key
                (service_id, key, secret)
                VALUES ($1, $2, $3)
                RETURNING
                id, service_id as "service_id: _", key, secret, whitelist, created_at"#,
            service_id as ServiceId,
            key,
            secret
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }
}
