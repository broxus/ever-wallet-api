use axum::{Extension, Json};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::axum_api::controllers::IdExtractor;
use crate::axum_api::requests::CreateAddressRequest;
use crate::axum_api::responses::AddressResponse;
use crate::axum_api::{ApiContext, Result};
use crate::models::Account;

pub async fn create_address(
    Json(req): Json<CreateAddressRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<AddressResponse>> {
    log::info!("Create address");

    let address = ctx
        .ton_service
        .create_address(&service_id, req.into())
        .await
        .unwrap();

    let res = AddressResponse::from(Ok(address.into()));
    log::info!("{:#?}", res);

    Ok(Json(res))
}
