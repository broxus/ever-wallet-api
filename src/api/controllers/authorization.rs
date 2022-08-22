use std::sync::Arc;

use axum::async_trait;
use axum::body::{boxed, Body, Full};
use axum::extract::{FromRequest, OriginalUri, RequestParts};
use axum::http::Request;
use axum::middleware::Next;
use axum::response::IntoResponse;
use http::{Method, StatusCode};

use crate::models::*;
use crate::services::*;

pub async fn verify_auth(
    req: Request<Body>,
    next: Next<Body>,
    auth_service: Arc<AuthService>,
) -> impl IntoResponse {
    match check_api_key(req, auth_service).await {
        Ok(req) => next.run(req).await,
        Err(err) => {
            log::error!("Failed to check auth. Err: {:?}", &err);
            Rejection("Failed to authorize".to_string(), StatusCode::UNAUTHORIZED).into_response()
        }
    }
}

async fn check_api_key(
    req: Request<Body>,
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

    let mut parts = RequestParts::new(req);

    let body = match method {
        Method::GET => String::new(),
        _ => {
            let body = parts.take_body();

            let bytes = match body {
                Some(b) => hyper::body::to_bytes(b).await?,
                None => anyhow::bail!("Request body is empty"),
            };
            parts.body_mut().replace(hyper::Body::from(bytes.clone()));

            let new_body = boxed(Full::from(bytes.clone()));
            let body_bytes = hyper::body::to_bytes(new_body).await?;

            String::from_utf8(body_bytes.to_vec())?
        }
    };

    let service_id = auth_service
        .authenticate(&api_key, &timestamp, &signature, &path, &body, real_ip)
        .await?;

    // Forward service id to request handler
    parts.extensions_mut().insert(IdExtractor(service_id));

    Ok(Request::from_request(&mut parts).await.expect("can't fail"))
}

pub struct IdExtractor(pub ServiceId);

#[async_trait]
impl<B> FromRequest<B> for IdExtractor
where
    B: Send, // required by `async_trait`
{
    type Rejection = Rejection;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let extensions = req.extensions();

        let id: Option<&IdExtractor> = extensions.get();
        match id {
            Some(service_id) => Ok(IdExtractor(service_id.0)),
            None => Err(Rejection(
                "Service id not found".to_string(),
                StatusCode::UNAUTHORIZED,
            )),
        }
    }
}

pub struct Rejection(String, StatusCode);

impl IntoResponse for Rejection {
    fn into_response(self) -> axum::response::Response {
        (self.1, self.0).into_response()
    }
}
