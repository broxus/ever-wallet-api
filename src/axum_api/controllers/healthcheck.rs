use std::time::{SystemTime, UNIX_EPOCH};

use axum::response::IntoResponse;

pub async fn get_healthcheck() -> impl IntoResponse {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_millis()
        .to_string()
}
