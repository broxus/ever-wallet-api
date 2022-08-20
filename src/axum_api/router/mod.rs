use axum::response::IntoResponse;
use axum::{routing::post, Extension, Router};
use http::StatusCode;
use std::sync::Arc;

use crate::axum_api::controllers::verify_auth;
use crate::axum_api::{controllers, ApiContext};
use crate::services::{AuthService, StorageHandler, TonService};

mod address;

pub fn router(
    auth_service: Arc<AuthService>,
    ton_service: Arc<dyn TonService>,
    memory_storage: Arc<StorageHandler>,
) -> Router {
    Router::new().nest(
        "/ton/v3",
        api_routes(auth_service, ton_service, memory_storage),
    )
}

fn api_routes(
    auth_service: Arc<AuthService>,
    ton_service: Arc<dyn TonService>,
    memory_storage: Arc<StorageHandler>,
) -> Router {
    Router::new()
        .nest("/address", address::router())
        .layer(axum::middleware::from_fn(move |req, next| {
            verify_auth(req, next, auth_service.clone())
        }))
        .layer(Extension(Arc::new(ApiContext {
            ton_service,
            memory_storage,
        })))
}
