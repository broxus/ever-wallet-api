use axum::{
    routing::{get, post},
    Router,
};

use crate::api::controllers;

pub fn router() -> Router {
    Router::new()
        .route(
            "/address/:address",
            get(controllers::get_token_address_balance),
        )
        .route(
            "/transactions/id/:internal_id",
            get(controllers::get_tokens_transactions_id),
        )
        .route(
            "/transactions/mh/:message_hash",
            get(controllers::get_tokens_transactions_mh),
        )
        .route(
            "/transactions/create",
            post(controllers::post_tokens_transactions_create),
        )
        .route(
            "/transactions/burn",
            post(controllers::post_tokens_transactions_burn),
        )
        .route(
            "/transactions/mint",
            post(controllers::post_tokens_transactions_mint),
        )
        .route("/events", post(controllers::post_tokens_events))
        .route("/events/mark", post(controllers::post_tokens_events_mark))
        .route("/whitelist", get(controllers::get_token_whitelist))
}
