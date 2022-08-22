use axum::{
    routing::{get, post},
    Router,
};

use crate::api::controllers;

pub fn router() -> Router {
    Router::new()
        .route("/", post(controllers::post_events))
        .route("/mark", post(controllers::post_events_mark))
        .route("/mark/all", post(controllers::post_events_mark_all))
        .route("/id/:id", get(controllers::get_events_id))
}
