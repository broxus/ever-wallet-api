use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::convert::Infallible;
use std::future::IntoFuture;

use anyhow::Context;
use axum::handler::Handler;
use axum::body::Body;
use axum::http::{Method};
use metrics::{describe_counter, describe_histogram};
use metrics_exporter_prometheus::Matcher;
use tower::ServiceBuilder;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::Span;
use axum::response::Response;
use axum::extract::Request;
use axum::serve::IncomingStream;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use tower_service::Service;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub listen_addr: SocketAddr,
    pub public_url: Option<String>,
}

impl Default for ApiConfig {
    #[inline]
    fn default() -> Self {
        Self {
            listen_addr: (Ipv4Addr::LOCALHOST, 8080).into(),
            public_url: None,
        }
    }
}

pub struct Api {
    serve_fn: Box<dyn FnOnce() -> BoxFuture<'static, std::io::Result<()>> + Send>,
}

impl Api {
pub async fn bind< M, S> (
    server_addr: SocketAddr,
    metrics_addr: Option<SocketAddr>,
    auth_service: Arc<AuthService>,
    ton_service: Arc<TonService>,
    memory_storage: Arc<StorageHandler>,
) 
 -> std::io::Result<Self>
    where
        M: for<'a> Service<IncomingStream<'a>, Error = Infallible, Response = S> + Send + 'static,
        S: Service<Request, Response = Response, Error = Infallible> + Clone + Send + 'static,
        for<'a> <M as Service<IncomingStream<'a>>>::Future: Send,
        S::Future: Send,
        {
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
            .fallback(controllers::handler_404);


        let listener = tokio::net::TcpListener::bind(server_addr).await.unwrap();

        let serve = axum::serve(listener, app);

        Ok(Self {
            serve_fn: Box::new(move || Box::pin(serve.into_future())),
        })
    }

    pub async fn serve(self) -> std::io::Result<()> {
        (self.serve_fn)().await
    }
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
