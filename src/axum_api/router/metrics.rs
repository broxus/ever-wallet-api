use axum::{routing::get, Router};

use crate::axum_api::controllers;

pub fn router() -> Router {
    Router::new().route("/", get(controllers::get_metrics))
}
