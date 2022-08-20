use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Context;
use axum::body::Body;
use axum::error_handling::HandleErrorLayer;
use axum::extract::{ContentLengthLimit, FromRequest};
use axum::handler::Handler;
use axum::http::Request;
use axum::middleware::{from_extractor, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{middleware, BoxError, Extension, Router};
use http::header::{HeaderName, AUTHORIZATION, CONTENT_TYPE};
use http::method::Method;
use http::{header, HeaderMap, HeaderValue, StatusCode};
use metrics::{
    describe_counter, describe_gauge, describe_histogram, gauge, histogram, increment_counter,
};
use metrics_exporter_prometheus::Matcher;
use reqwest::Url;
use serde_json::Value;
use tokio::time::Instant;
use tower::ServiceBuilder;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::metrics::InFlightRequestsLayer;
use tower_http::trace::TraceLayer;
use tracing::Span;

use crate::axum_api::controllers::handler_404;
use crate::axum_api::router::router;
use crate::services::{AuthService, StorageHandler, TonService};

mod controllers;
mod error;
mod requests;
mod responses;
mod router;

const EXPONENTIAL_SECONDS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

type Result<T, E = error::Error> = std::result::Result<T, E>;

pub async fn http_service(
    server_addr: SocketAddr,
    metrics_addr: Option<SocketAddr>,
    auth_service: Arc<AuthService>,
    ton_service: Arc<dyn TonService>,
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

    let app = router(auth_service, ton_service, memory_storage)
        .layer(
            ServiceBuilder::new().layer(
                CorsLayer::new()
                    .allow_headers(AllowHeaders::any())
                    .allow_origin(AllowOrigin::mirror_request())
                    .allow_methods(AllowMethods::list([
                        Method::GET,
                        Method::POST,
                        Method::OPTIONS,
                    ])),
            ),
        )
        .fallback(handler_404.into_service());

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
    ton_service: Arc<dyn TonService>,
    memory_storage: Arc<StorageHandler>,
}

/*async fn router(
    auth_service: Arc<AuthService>,
    ton_service: Arc<dyn TonService>,
    memory_storage: Arc<StorageHandler>,
) -> Router {
    describe_gauge!("in_flight_requests", "number of inflight requests");
    let (in_flight_requests_layer, counter) = InFlightRequestsLayer::pair();
    tokio::spawn(async {
        counter
            .run_emitter(Duration::from_secs(5), |count| async move {
                gauge!("in_flight_requests", count as f64)
            })
            .await;
    });

    router::router()
        .layer(axum::middleware::from_fn(move |req, next| {
            auth(req, next, auth_service.clone())
        }))
        .layer(Extension(Arc::new(ApiContext {
            ton_service,
            memory_storage,
        })))

    /*Router::new()
    .route("/", get(health_check))
    .route("/address")
    .nest("/create")
    .route(
        "/rpc",
        post(jrpc_router).layer(Extension(Arc::new(State {
            ton_service,
            auth_service,
            memory_storage,
        }))),
    )
    .layer(
        ServiceBuilder::new()
            .layer(in_flight_requests_layer)
            .layer(CompressionLayer::new())
            .layer(DecompressionLayer::new())
            .layer(
                CorsLayer::new()
                    .allow_headers(AllowHeaders::list([
                        AUTHORIZATION,
                        CONTENT_TYPE,
                        HeaderName::from_static("api-key"),
                    ]))
                    .allow_origin(AllowOrigin::any())
                    .allow_methods(AllowMethods::list([
                        Method::GET,
                        Method::POST,
                        Method::OPTIONS,
                        Method::PUT,
                    ])),
            )
            .layer(HandleErrorLayer::new(|_: BoxError| async {
                // converts timeout error to status code
                StatusCode::REQUEST_TIMEOUT
            }))
            .layer(TimeoutLayer::new(Duration::from_secs(10))),
    )*/
}*/

/*async fn auth(
    req: Request<Body>,
    next: Next<Body>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut res = next.run(req).await;
    res.extensions_mut().insert(IdWrapper(1));

    Ok(res)
}*/

async fn health_check() -> impl IntoResponse {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_millis()
        .to_string()
}

/*type RpcExtractor = ContentLengthLimit<axum_jrpc::JsonRpcExtractor, 24576>;

macro_rules! reject_on_error {
    ($expr:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => return dont_cache(e),
        }
    };
}

async fn jrpc_router(
    Extension(ctx): Extension<Arc<State>>,
    ContentLengthLimit(axum_jrpc::JsonRpcExtractor { id, method, parsed }): RpcExtractor,
) -> impl IntoResponse {
    let start = Instant::now();

    let response = match method.as_str() {
        "getContractState" => contract_response(jrpc_req(&ctx.client, id, &method, &parsed).await),
        "sendMessage" | "getLatestKeyBlock" => {
            dont_cache(jrpc_req(&ctx.client, id, &method, &parsed).await)
        }
        "getTransactionsList" => {
            match controllers::get_raw_transactions_list(
                &ctx.db,
                reject_on_error!(parse_params(id, parsed)),
            )
            .await
            {
                Ok((result, cacheable)) if cacheable => {
                    cache_for(JsonRpcResponse::success(id, result), 86400)
                }
                Ok((result, _)) => dont_cache(JsonRpcResponse::success(id, result)),
                Err(e) => {
                    log::error!("Failed to get transactions list");
                    dont_cache(JsonRpcResponse::error(id, e.into()))
                }
            }
        }
        "getTransaction" => {
            match controllers::get_raw_transaction(
                &ctx.db,
                reject_on_error!(parse_params(id, parsed)),
            )
            .await
            {
                Ok(Some(result)) => cache_for(JsonRpcResponse::success(id, result), 86400),
                Ok(None) => dont_cache(JsonRpcResponse::success(id, Value::Null)),
                Err(e) => {
                    log::error!("Failed to get transaction");
                    dont_cache(JsonRpcResponse::error(id, e.into()))
                }
            }
        }
        "getAccountsByCodeHash" => {
            match controllers::get_accounts_by_code_hash(
                &ctx.db,
                reject_on_error!(parse_params(id, parsed)),
            )
            .await
            {
                Ok(result) => dont_cache(JsonRpcResponse::success(id, result)),
                Err(e) => {
                    log::error!("Failed to get accounts by code hash");
                    dont_cache(JsonRpcResponse::error(id, e.into()))
                }
            }
        }
        method => dont_cache(method_not_found(id, method)),
    };

    let elapsed = start.elapsed();
    histogram!("jrpc_execution_time_seconds", elapsed, "method" => method.clone());
    increment_counter!("jrpc_requests_processed", "method" => method);

    response
}

async fn jrpc_req<T>(client: &LoadBalancedRpc, id: i64, method: &str, params: T) -> JsonRpcResponse
where
    T: serde::Serialize,
{
    client.request(JrpcRequest { id, method, params }).await
}

pub fn parse_params<T>(id: i64, parsed: Value) -> Result<T, JsonRpcResponse>
where
    for<'de> T: serde::de::Deserialize<'de>,
{
    serde_json::from_value(parsed).map_err(|e| {
        let error = axum_jrpc::error::JsonRpcError::new(
            axum_jrpc::error::JsonRpcErrorReason::InvalidParams,
            e.to_string(),
            Value::Null,
        );
        JsonRpcResponse::error(id, error)
    })
}

pub fn method_not_found(id: i64, method: &str) -> JsonRpcResponse {
    let error = axum_jrpc::error::JsonRpcError::new(
        axum_jrpc::error::JsonRpcErrorReason::MethodNotFound,
        format!("Method `{method}` not found"),
        Value::Null,
    );
    JsonRpcResponse::error(id, error)
}

fn contract_response(value: JsonRpcResponse) -> (HeaderMap, JsonRpcResponse) {
    let val = match &value.result {
        JsonRpcAnswer::Result(r) => r,
        JsonRpcAnswer::Error(_) => return dont_cache(value),
    };

    let should_cache = if let Value::Object(map) = &val {
        matches!(map.get("type"), Some(Value::String(s)) if s == "exists")
    } else {
        false
    };

    if should_cache {
        cache_for(value, 3)
    } else {
        dont_cache(value)
    }
}

fn dont_cache<T>(response: T) -> (HeaderMap, T)
where
    T: IntoResponse,
{
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    (headers, response)
}

fn cache_for<T>(response: T, time: u32) -> (HeaderMap, T)
where
    T: IntoResponse,
{
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_str(&format!("public,max-age={}", time)).expect("valid cache control"),
    );
    (headers, response)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn correct_cache_headers() {
        let client = LoadBalancedRpc::new(
            ["https://jrpc.everwallet.net/rpc".parse().unwrap()],
            LoadBalancedRpcOptions {
                prove_interval: Duration::from_secs(1),
            },
        )
        .await;

        // Existing contract
        let data = jrpc_req(&client,1,"getContractState",serde_json::json!({"address":"0:376d5f09522feb8d4d44b6b4b98cb73887a16def049ada636a2fc38fbaa1b56d"})).await;
        assert_eq!(
            contract_response(data).0.get(header::CACHE_CONTROL),
            Some(&HeaderValue::from_static("public,max-age=3"))
        );

        // Non-existing contract
        let data = jrpc_req(&client,1,"getContractState",serde_json::json!({"address":"0:1234123412341234123412341234123412341234123412341234123412341234"})).await;
        assert_eq!(
            contract_response(data).0.get(header::CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );
    }
}
*/
