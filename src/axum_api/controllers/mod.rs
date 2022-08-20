use std::sync::Arc;

use anyhow::Result;
use axum::extract::FromRequest;
use axum::{Extension, Json};
use serde_json::{json, Value};

use crate::axum_api::ApiContext;
use axum::response::IntoResponse;
use axum::{
    async_trait,
    body::{boxed, Body, Full},
    extract::RequestParts,
    http::{Request, StatusCode},
    middleware::Next,
    routing::put,
    Router,
};

use crate::models::ServiceId;

pub use self::address::*;
pub use self::auth::*;

mod address;
mod auth;

pub async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND).into_response()
}

/*pub struct IdWrapper(pub i32);

pub struct Id(pub i32);

#[async_trait]
impl<B> FromRequest<B> for Id
where
    B: Send, // required by `async_trait`
{
    type Rejection = Rejection;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let extensions = req.extensions();
        let id: Option<&IdWrapper> = extensions.get();
        match id {
            Some(a) => Ok(Self(a.0)),
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
