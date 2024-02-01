use anyhow::Result;

use crate::models::*;
use crate::sqlx_client::*;

impl SqlxClient {
    pub async fn get_callback(&self, service_id: ServiceId) -> Result<String> {
        sqlx::query!(
            r#"SELECT callback
                FROM api_service_callback
                WHERE service_id = $1"#,
            service_id as ServiceId,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
        .map(|x| x.callback)
    }

    pub async fn set_callback(&self, payload: ApiServiceCallbackDb) ->  Result<()> {
        sqlx::query!(
                r#"INSERT INTO api_service_callback
                (id, service_id, callback, created_at) VALUES ($1, $2, $3, $4)
                ON CONFLICT (service_id)
                DO UPDATE SET callback = $3"#,
                payload.id,
                payload.service_id as ServiceId,
                payload.callback,
                payload.created_at,
            )
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
