use std::convert::Infallible;
use std::sync::Arc;

use axum::response::IntoResponse;
use axum::routing::get_service;
use axum::{Extension, Router};
use tower::service_fn;

use crate::api::*;
use crate::services::*;

mod address;
mod events;
mod metrics;
mod misc;
mod tokens;
mod transactions;

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
}

fn api_router(
    auth_service: Arc<AuthService>,
    ton_service: Arc<TonService>,
    memory_storage: Arc<StorageHandler>,
) -> Router {
    Router::new()
        .nest("/address", address::router())
        .nest("/events", events::router())
        .nest("/tokens", tokens::router())
        .nest("/misc", misc::router())
        .nest("/metrics", metrics::router())
        .nest("/transactions", transactions::router())
        .layer(axum::middleware::from_fn(move |req, next| {
            controllers::verify_auth(req, next, auth_service.clone())
        }))
        .layer(Extension(Arc::new(ApiContext {
            ton_service,
            memory_storage,
        })))
}
