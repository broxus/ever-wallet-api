use axum::{
    routing::{get, post},
    Router,
};

use crate::api::controllers;

pub fn router() -> Router {
    Router::new()
        .route("/", post(controllers::post_transactions))
        .route("/create", post(controllers::post_transactions_create))
        .route("/confirm", post(controllers::post_transactions_confirm))
        .route("/id/:id", get(controllers::get_transactions_id))
        .route("/h/:hash", get(controllers::get_transactions_h))
        .route("/mh/:message_hash", get(controllers::get_transactions_mh))
}
