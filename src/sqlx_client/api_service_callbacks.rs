use crate::models::account_enums::AccountType;
use crate::models::address::CreateAddressInDb;
use crate::models::service_id::ServiceId;
use crate::models::sqlx::AddressDb;
use crate::prelude::ServiceError;
use crate::sqlx_client::SqlxClient;

impl SqlxClient {
    pub async fn get_callback(&self, service_id: ServiceId) -> Result<String, ServiceError> {
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
}
