use axum::{Extension, Json};

use crate::api::responses::*;
use crate::api::*;

pub async fn get_blockchain_info(
    Extension(ctx): Extension<Arc<ApiContext>>,
) -> Result<Json<MetricsResponse>> {
    let metrics = ctx.ton_service.get_metrics().await?;

    Ok(Json(MetricsResponse::from(metrics)))
}
