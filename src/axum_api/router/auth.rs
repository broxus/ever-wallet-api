/*use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::models::ServiceId;
use anyhow::Context;
use axum::async_trait;
use axum::body::Body;
use axum::error_handling::HandleErrorLayer;
use axum::extract::{ContentLengthLimit, FromRequest, RequestParts};
use axum::http::Request;
use axum::middleware::Next;
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
use tower::timeout::TimeoutLayer;
use tower::ServiceBuilder;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::metrics::InFlightRequestsLayer;

use crate::services::{AuthService, StorageHandler, TonService};

pub async fn auth(
    req: Request<Body>,
    next: Next<Body>,
    auth_service: Arc<AuthService>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut res = next.run(req).await;

    res.extensions_mut()
        .insert(IdExtractor(ServiceId::new(uuid::Uuid::new_v4())));

    Ok(res)
}

pub struct IdExtractor(pub ServiceId);

struct IdWrapper(pub ServiceId);

#[async_trait]
impl<B> FromRequest<B> for IdExtractor
where
    B: Send, // required by `async_trait`
{
    type Rejection = Rejection;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let extensions = req.extensions();
        let id: Option<&IdWrapper> = extensions.get();
        match id {
            Some(a) => Ok(IdExtractor(a.0)),
            None => Err(Rejection("(".to_string(), StatusCode::IM_A_TEAPOT)),
        }
    }
}

pub struct Rejection(String, StatusCode);

impl IntoResponse for Rejection {
    fn into_response(self) -> axum::response::Response {
        (self.1, self.0).into_response()
    }
}
*/
