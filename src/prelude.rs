pub use futures::prelude::*;

use r2d2::{Pool, PooledConnection};
use sqlx::Error;

#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
    #[error("auth error: `{0}`")]
    Auth(String),
    #[error("`{0}` not found")]
    NotFound(String),
    #[error("Wrong Input - `{0}`")]
    WrongInput(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<sqlx::Error> for ServiceError {
    fn from(e: Error) -> Self {
        match e {
            Error::RowNotFound => ServiceError::NotFound("database row".to_string()),
            _ => ServiceError::Other(anyhow::Error::new(e)),
        }
    }
}

impl From<reqwest::Error> for ServiceError {
    fn from(e: reqwest::Error) -> Self {
        ServiceError::Other(anyhow::Error::new(e))
    }
}

impl<'a> From<&'a ServiceError> for http::Response<hyper::Body> {
    fn from(err: &'a ServiceError) -> http::Response<hyper::Body> {
        use http::status::StatusCode;

        match err {
            ServiceError::NotFound(_) => http::Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "application/json")
                .body(
                    serde_json::json!({
                        "code": "404",
                        "description": "Not found",
                        "message": err.to_string()
                    })
                    .to_string()
                    .into(),
                )
                .expect("failed to build errors response"),
            ServiceError::Other(_) => http::Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(
                    serde_json::json!({
                        "description": "Internal server error",
                    })
                    .to_string()
                    .into(),
                )
                .expect("failed to build errors response"),

            ServiceError::Auth(_) => http::Response::builder()
                .status(StatusCode::FORBIDDEN)
                .header("Content-Type", "application/json")
                .body(
                    serde_json::json!({
                        "code": "403",
                        "description": "Request forbidden",
                        "message": err.to_string()
                    })
                    .to_string()
                    .into(),
                )
                .unwrap(),

            ServiceError::WrongInput(_) => http::Response::builder()
                .status(StatusCode::UNPROCESSABLE_ENTITY)
                .header("Content-Type", "application/json")
                .body(
                    serde_json::json!({
                        "code": "422",
                        "description": "Request forbidden",
                        "message": err.to_string()
                    })
                    .to_string()
                    .into(),
                )
                .unwrap(),
        }
    }
}

impl warp::reject::Reject for ServiceError {}

pub const TOKEN_FEE: &str = "500000000";
pub const MAX_LIMIT_SEARCH: i64 = 100i64;
