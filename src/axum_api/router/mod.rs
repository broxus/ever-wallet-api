use std::convert::Infallible;
use std::sync::Arc;

use axum::response::IntoResponse;
use axum::routing::get_service;
use axum::{routing::get, Extension, Router};
use tower::service_fn;

use crate::axum_api::*;
use crate::services::*;

mod address;
mod metrics;
mod tokens;

const API_PREFIX: &str = "/ton/v3";

pub fn router(
    auth_service: Arc<AuthService>,
    ton_service: Arc<TonService>,
    memory_storage: Arc<StorageHandler>,
) -> Router {
    Router::new()
        .nest(
            API_PREFIX,
            api_router(auth_service, ton_service, memory_storage),
        )
        .route(
            "/",
            get_service(service_fn(|_: _| async move {
                Ok::<_, Infallible>(
                    controllers::swagger("https://ton-api.broxus.com/ton/v3").into_response(),
                )
            })),
        )
        .route(
            "/swagger.yaml",
            get_service(service_fn(|_: _| async move {
                Ok::<_, Infallible>(
                    controllers::swagger("https://ton-api.broxus.com/ton/v3").into_response(),
                )
            })),
        )
        .route("/healthcheck", get(controllers::get_healthcheck))
}

fn api_router(
    auth_service: Arc<AuthService>,
    ton_service: Arc<TonService>,
    memory_storage: Arc<StorageHandler>,
) -> Router {
    Router::new()
        .nest("/address", address::router())
        .nest("/tokens", tokens::router())
        .nest("/metrics", metrics::router())
        .layer(axum::middleware::from_fn(move |req, next| {
            controllers::verify_auth(req, next, auth_service.clone())
        }))
        .layer(Extension(Arc::new(ApiContext {
            ton_service,
            memory_storage,
        })))
}
