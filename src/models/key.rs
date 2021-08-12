use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::service_id::ServiceId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Key {
    pub id: Uuid,
    pub service_id: ServiceId,
    pub key: String,
    pub secret: String,
    pub whitelist: Option<serde_json::Value>,
    pub created_at: NaiveDateTime,
}
