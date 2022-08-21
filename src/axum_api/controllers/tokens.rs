use axum::extract::Path;
use axum::{Extension, Json};

use crate::axum_api::controllers::*;
use crate::axum_api::responses::*;
use crate::axum_api::*;
use crate::models::*;

pub async fn get_token_address_balance(
    Path(address): Path<Address>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<TokenBalanceResponse>> {
    let addresses = ctx
        .ton_service
        .get_token_address_balance(&service_id, &address)
        .await
        .map(|a| {
            a.into_iter()
                .map(|(a, b)| TokenBalanceDataResponse::new(a, b))
                .collect::<Vec<TokenBalanceDataResponse>>()
        });

    Ok(Json(TokenBalanceResponse::from(addresses)))
}
