use axum::{routing::post, Router};

use crate::api::controllers;

pub fn router() -> Router {
    Router::new()
        .route("/read-contract", post(controllers::post_read_contract))
        .route("/encode-into-cell", post(controllers::post_encode_tvm_cell))
        .route(
            "/prepare-message",
            post(controllers::post_prepare_generic_message),
        )
        .route(
            "/send-signed-message",
            post(controllers::post_send_signed_message),
        )
}
