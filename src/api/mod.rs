use std::future::IntoFuture;
use std::net::SocketAddr;
use std::sync::Arc;

use aide::openapi::{Info, OpenApi, Server};
use aide::transform::{TransformOpenApi, TransformPathItem};
use anyhow::Context;
use axum::body::Body;
use axum::extract::Request;
use axum::http::Method;
use futures_util::future::BoxFuture;
use metrics::{describe_counter, describe_histogram};
use metrics_exporter_prometheus::Matcher;
use schemars::schema::{InstanceType, SchemaObject};
use tower::ServiceBuilder;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::Span;

use crate::services::{AuthService, StorageHandler, TonService};

pub use self::error::*;

mod controllers;
mod docs;
mod error;
mod requests;
mod responses;
mod router;

const EXPONENTIAL_SECONDS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

type Result<T, E = Error> = std::result::Result<T, E>;

pub struct Api {
    serve_fn: Box<dyn FnOnce() -> BoxFuture<'static, std::io::Result<()>> + Send>,
}

impl Api {
    pub async fn bind(
        server_addr: SocketAddr,
        public_url: Option<String>,
        metrics_addr: Option<SocketAddr>,
        auth_service: Arc<AuthService>,
        ton_service: Arc<TonService>,
        memory_storage: Arc<StorageHandler>,
    ) -> std::io::Result<Self> {
        describe_counter!("requests_processed", "number of requests processed");
        describe_histogram!(
            "execution_time_seconds",
            metrics::Unit::Milliseconds,
            "execution time of request handler"
        );
        if let Some(metrics_addr) = metrics_addr {
            if let Err(e) = install_monitoring(metrics_addr) {
                tracing::error!("Failed to install monitoring: {e:?}");
            }
        }
        let mut api =
            get_open_api(public_url.unwrap_or_else(|| "http://localhost:8080".to_string()));

        let app = router::router(auth_service, ton_service, memory_storage)
            .finish_api_with(&mut api, api_docs)
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
            .layer(TraceLayer::new_for_http().on_request(
                |request: &Request<Body>, _span: &Span| {
                    tracing::info!("started {} {}", request.method(), request.uri().path())
                },
            ))
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

#[derive(Clone)]
pub struct ApiContext {
    ton_service: Arc<TonService>,
    memory_storage: Arc<StorageHandler>,
}

fn taged(tag: &'static str) -> impl FnOnce(TransformPathItem) -> TransformPathItem {
    |item| item.tag(tag)
}

fn get_open_api(url: String) -> OpenApi {
    OpenApi {
        info: Info {
            description: Some("Tycho Wallet API".to_string()),
            ..Info::default()
        },
        servers: vec![Server {
            url,
            description: Some("Production".to_string()),
            variables: Default::default(),
            extensions: Default::default(),
        }],
        ..OpenApi::default()
    }
}

fn api_docs(api: TransformOpenApi) -> TransformOpenApi {
    api.title("Tycho Wallet API")
        .summary("Tycho Wallet indexer")
}

pub(super) fn int_schema(_: &mut schemars::SchemaGenerator) -> schemars::schema::Schema {
    let object_schema = schemars::schema::SchemaObject { instance_type: Some(InstanceType::Number.into()), ..Default::default() };
    object_schema.into()
}
pub(super) fn any_schema(_: &mut schemars::SchemaGenerator) -> schemars::schema::Schema {
    let object_schema = SchemaObject::default();
    object_schema.into()
}
