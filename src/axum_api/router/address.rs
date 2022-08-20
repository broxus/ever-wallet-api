use axum::{routing::post, Router};

use crate::axum_api::controllers::create_address;

pub fn router() -> Router {
    Router::new().route("/create", post(create_address))
}
