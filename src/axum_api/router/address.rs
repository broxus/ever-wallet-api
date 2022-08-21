use axum::{
    routing::{get, post},
    Router,
};

use crate::axum_api::controllers;

pub fn router() -> Router {
    Router::new()
        .route("/check", post(controllers::post_address_check))
        .route("/create", post(controllers::post_address_create))
        .route("/:address", get(controllers::get_address_balance))
        .route("/:address/info", get(controllers::get_address_info))
}
