use std::sync::Arc;

use aide::{OperationInput, OperationOutput};
use axum::async_trait;
use axum::body::Body;
use axum::extract::{FromRequest, FromRequestParts, OriginalUri};
use axum::http::request::Parts;
use axum::http::Request;
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use schemars::JsonSchema;

use crate::api::int_schema;
use crate::models::*;
use crate::services::*;

pub async fn verify_auth(
    req: Request<Body>,
    next: Next,
    auth_service: Arc<AuthService>,
) -> impl IntoResponse {
    match check_api_key(req, auth_service).await {
        Ok(req) => next.run(req).await,
        Err(err) => {
            tracing::error!("Failed to check auth. Err: {:?}", &err);
            Rejection {
                reason: "Failed to authorize".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            }
            .into_response()
        }
    }
}

async fn check_api_key(
    mut req: Request<Body>,
    auth_service: Arc<AuthService>,
) -> anyhow::Result<Request<Body>> {
    let api_key_opt = req.headers().get("api-key");
    let timestamp_opt = req.headers().get("timestamp");
    let signature_opt = req.headers().get("sign");

    let (api_key, timestamp, signature) = match (api_key_opt, timestamp_opt, signature_opt) {
        (Some(api_key), Some(timestamp), Some(signature)) => (
            api_key
                .to_str()
                .map(|x| x.to_string())
                .map_err(|_| anyhow::Error::msg("Failed to read API-KEY header"))?,
            timestamp
                .to_str()
                .map(|x| x.to_string())
                .map_err(|_| anyhow::Error::msg("Failed to read timestamp header"))?,
            signature
                .to_str()
                .map(|x| x.to_string())
                .map_err(|_| anyhow::Error::msg("Failed to read signature header"))?,
        ),
        _ => anyhow::bail!("One or more auth headers are missing"),
    };

    let real_ip_opt = req.headers().get("x-real-ip");
    let real_ip = match real_ip_opt {
        Some(real_ip) => Some(
            real_ip
                .to_str()
                .map(|x| x.to_string())
                .map_err(|_| anyhow::Error::msg("Failed to read x-real-ip header"))?,
        ),
        None => None,
    };

    let path = if let Some(path) = req.extensions().get::<OriginalUri>() {
        path.0.path().to_owned()
    } else {
        req.uri().path().to_owned()
    };

    let method = req.method().clone();

    let body = match method {
        Method::GET => String::new(),
        _ => {
            let (parts, body) = req.into_parts();
            let body_bytes = axum::body::to_bytes(body, 100000).await?;
            req = Request::from_parts(parts, Body::from(body_bytes.to_vec()));
            String::from_utf8(body_bytes.to_vec())?
        }
    };

    let service_id = auth_service
        .authenticate(&api_key, &timestamp, &signature, &path, &body, real_ip)
        .await?;

    // Forward service id to request handler
    req.extensions_mut().insert(IdExtractor(service_id));

    Ok(Request::from_request(req, &auth_service)
        .await
        .expect("can't fail"))
}

#[derive(Debug, Clone)]
pub struct IdExtractor(pub ServiceId);

#[async_trait]
impl<S> FromRequestParts<S> for IdExtractor
where
    S: Send + Sync,
{
    type Rejection = Rejection;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<IdExtractor, Self::Rejection> {
        let id: Option<&IdExtractor> = parts.extensions.get();
        match id {
            Some(service_id) => Ok(IdExtractor(service_id.0)),
            None => Err(Rejection {
                reason: "Service id not found".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            }),
        }
    }
}

#[derive(Debug, Clone, JsonSchema)]
pub struct Rejection {
    pub reason: String,
    #[schemars(schema_with = "int_schema")]
    pub status_code: StatusCode,
}

impl IntoResponse for Rejection {
    fn into_response(self) -> axum::response::Response {
        (self.status_code, self.reason).into_response()
    }
}

impl OperationOutput for Rejection {
    type Inner = Self;
}

impl OperationInput for IdExtractor {}
