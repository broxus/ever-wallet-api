use axum::{Extension, Json};

use crate::api::responses::*;
use crate::api::*;

pub async fn get_blockchain_info(
    Extension(ctx): Extension<Arc<ApiContext>>,
) -> Result<Json<BlockchainInfoResponse>> {
    let info = ctx.ton_service.get_blockchain_info().await?;

    Ok(Json(BlockchainInfoResponse::from(info)))
}
