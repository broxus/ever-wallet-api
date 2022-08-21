use axum::{Extension, Json};

use crate::axum_api::responses::*;
use crate::axum_api::*;

pub async fn get_metrics(
    Extension(ctx): Extension<Arc<ApiContext>>,
) -> Result<Json<MetricsResponse>> {
    let metrics = ctx.ton_service.get_metrics().await?;
    Ok(Json(MetricsResponse::from(metrics)))
}
