use aide::axum::{routing::get_with, ApiRouter};
use axum::Json;

use crate::api::{controllers, responses::MetricsResponse, taged, ApiContext};

pub fn router() -> ApiRouter<ApiContext> {
    ApiRouter::new().api_route_with(
        "/",
        get_with(controllers::get_ton_metrics, |op| {
            op.response::<200, Json<MetricsResponse>>()
        }),
        taged("metrics"),
    )
}
