use axum::{routing::get, Router};

use crate::axum_api::controllers;

pub fn router() -> Router {
    Router::new().route(
        "/address/:address",
        get(controllers::get_token_address_balance),
    )
}
