use axum::{
    routing::{get, post},
    Router,
};

use crate::axum_api::controllers;

pub fn router() -> Router {
    Router::new()
        .route(
            "/address/:address",
            get(controllers::get_token_address_balance),
        )
        .route("/events", post(controllers::post_tokens_events))
        .route("/events/mark", post(controllers::post_tokens_events_mark))
}
