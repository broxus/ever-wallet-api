use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use axum::handler::Handler;
use http::method::Method;
use http::Request;
use hyper::Body;
use metrics::{describe_counter, describe_histogram};
use metrics_exporter_prometheus::Matcher;
use tower::ServiceBuilder;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::Span;

use crate::services::{AuthService, StorageHandler, TonService};

pub use self::error::*;

mod controllers;
mod error;
mod requests;
mod responses;
mod router;

const EXPONENTIAL_SECONDS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

type Result<T, E = Error> = std::result::Result<T, E>;

pub async fn http_service(
    server_addr: SocketAddr,
    metrics_addr: Option<SocketAddr>,
    auth_service: Arc<AuthService>,
    ton_service: Arc<TonService>,
    memory_storage: Arc<StorageHandler>,
) {
    describe_counter!("requests_processed", "number of requests processed");
    describe_histogram!(
        "execution_time_seconds",
        metrics::Unit::Milliseconds,
        "execution time of request handler"
    );
    if let Some(metrics_addr) = metrics_addr {
        if let Err(e) = install_monitoring(metrics_addr) {
            log::error!("Failed to install monitoring: {e:?}");
        }
    }

    let app = router::router(auth_service, ton_service, memory_storage)
        .layer(
            ServiceBuilder::new().layer(
                CorsLayer::new()
                    .allow_headers(AllowHeaders::any())
                    .allow_origin(AllowOrigin::any())
                    .allow_methods(AllowMethods::list([
                        Method::GET,
                        Method::POST,
                        Method::OPTIONS,
                    ])),
            ),
        )
        .layer(
            TraceLayer::new_for_http().on_request(|request: &Request<Body>, _span: &Span| {
                tracing::info!("started {} {}", request.method(), request.uri().path())
            }),
        )
        .fallback(controllers::handler_404.into_service());

    axum::Server::bind(&server_addr)
        .serve(app.into_make_service())
        .await
        .context("Failed to start HTTP server")
        .unwrap();
}

fn install_monitoring(metrics_addr: SocketAddr) -> anyhow::Result<()> {
    metrics_exporter_prometheus::PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("execution_time_seconds".to_string()),
            EXPONENTIAL_SECONDS,
        )
        .context("Failed setting bucket")?
        .with_http_listener(metrics_addr)
        .install()
        .context("Failed installing metrics exporter")
}

pub struct ApiContext {
    ton_service: Arc<TonService>,
    memory_storage: Arc<StorageHandler>,
}
