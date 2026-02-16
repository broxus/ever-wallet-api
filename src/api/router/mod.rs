use std::sync::Arc;
use std::time::Duration;

use aide::axum::ApiRouter;
use metrics::{describe_gauge, gauge};
use tower_http::metrics::InFlightRequestsLayer;

use crate::api::*;
use crate::services::*;

mod address;
mod blockchain;
mod events;
mod misc;
mod tokens;
mod ton_metrics;
mod transactions;

const API_PREFIX: &str = "/ton/v3";

pub fn router(
    auth_service: Arc<AuthService>,
    ton_service: Arc<TonService>,
    memory_storage: Arc<StorageHandler>,
) -> ApiRouter {
    describe_gauge!("in_flight_requests", "number of inflight requests");
    let (in_flight_requests_layer, counter) = InFlightRequestsLayer::pair();
    let in_flight_gauge = gauge!("in_flight_requests");
    tokio::spawn(async move {
        counter
            .run_emitter(Duration::from_secs(5), move |count| {
                let gauge = in_flight_gauge.clone();
                async move {
                    gauge.set(count as f64)
                }
            })
            .await;
    });

    aide::axum::ApiRouter::new()
        .nest_api_service("/docs", docs::route())
        .nest_api_service(
            API_PREFIX,
            api_router(auth_service, ton_service, memory_storage),
        )
        .layer(in_flight_requests_layer)
}

fn api_router(
    auth_service: Arc<AuthService>,
    ton_service: Arc<TonService>,
    memory_storage: Arc<StorageHandler>,
) -> ApiRouter {
    aide::axum::ApiRouter::new()
        .nest("/blockchain", blockchain::router())
        .nest("/address", address::router())
        .nest("/events", events::router())
        .nest("/tokens", tokens::router())
        .nest("/misc", misc::router())
        .nest("/transactions", transactions::router())
        .nest("/metrics", ton_metrics::router())
        .layer(axum::middleware::from_fn(move |req, next| {
            controllers::verify_auth(req, next, auth_service.clone())
        }))
        .with_state(ApiContext {
            ton_service,
            memory_storage,
        })
}
