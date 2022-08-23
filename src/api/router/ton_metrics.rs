use axum::{routing::get, Router};

use crate::api::controllers;

pub fn router() -> Router {
    Router::new().route("/", get(controllers::get_ton_metrics))
}
