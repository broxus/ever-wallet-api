use aide::axum::{routing::get_with, ApiRouter};
use axum::Json;

use crate::api::{controllers, responses::BlockchainInfoResponse, taged, ApiContext};

pub fn router() -> ApiRouter<ApiContext> {
    ApiRouter::new().api_route_with(
        "/",
        get_with(controllers::get_blockchain_info, |op| {
            op.response::<200, Json<BlockchainInfoResponse>>()
        }),
        taged("blockchain"),
    )
}
