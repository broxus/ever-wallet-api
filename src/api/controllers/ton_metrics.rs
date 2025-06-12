use axum::extract::State;
use axum::Json;

use crate::api::responses::*;
use crate::api::*;

pub async fn get_ton_metrics(State(ctx): State<ApiContext>) -> Result<Json<MetricsResponse>> {
    let metrics = ctx.ton_service.get_metrics().await?;

    Ok(Json(MetricsResponse::from(metrics)))
}
