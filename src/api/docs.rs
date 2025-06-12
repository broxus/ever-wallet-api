use std::sync::Arc;

use aide::axum::routing::get;
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::openapi::OpenApi;
use aide::scalar::Scalar;
use axum::response::IntoResponse;
use axum::{Extension, Json};

pub fn route() -> ApiRouter {
    // We infer the return types for these routes
    // as an example.
    //
    // As a result, the `serve_redoc` route will
    // have the `text/html` content-type correctly set
    // with a 200 status.
    aide::gen::infer_responses(true);

    let router = ApiRouter::new()
        .route(
            "/",
            get(Scalar::new("/docs/private/api.json")
                .with_title("tycho wallet api")
                .axum_handler()),
        )
        .route("/private/api.json", get(serve_docs));

    // Afterwards we disable response inference because
    // it might be incorrect for other routes.
    aide::gen::infer_responses(false);

    router
}

pub async fn serve_docs(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoApiResponse {
    Json(api).into_response()
}
