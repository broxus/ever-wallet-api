use axum::extract::Path;
use axum::{Extension, Json};
use tokio::time::Instant;

use metrics::{histogram, increment_counter};

use crate::api::controllers::*;
use crate::api::requests::*;
use crate::api::responses::*;
use crate::api::*;
use crate::models::*;

pub async fn post_address_create(
    Json(req): Json<CreateAddressRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<AddressResponse>> {
    let start = Instant::now();

    let address = ctx
        .ton_service
        .create_address(&service_id, req.into())
        .await
        .map(From::from);

    let elapsed = start.elapsed();
    histogram!("execution_time_seconds", elapsed, "method" => "createAddress");
    increment_counter!("requests_processed", "method" => "createAddress");

    Ok(Json(AddressResponse::from(address)))
}

pub async fn post_address_add(
    Json(req): Json<AddAddressRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<AddressResponse>> {
    let start = Instant::now();

    let address = ctx
        .ton_service
        .add_address(&service_id, req.into())
        .await
        .map(From::from);

    let elapsed = start.elapsed();
    histogram!("execution_time_seconds", elapsed, "method" => "addAddress");
    increment_counter!("requests_processed", "method" => "addAddress");

    Ok(Json(AddressResponse::from(address)))
}

pub async fn post_address_check(
    Json(req): Json<AddressCheckRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
) -> Result<Json<CheckedAddressResponse>> {
    let address = ctx
        .ton_service
        .check_address(req.address)
        .await
        .map(AddressValidResponse::new);

    Ok(Json(CheckedAddressResponse::from(address)))
}

pub async fn get_address_balance(
    Path(address): Path<Address>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<AddressBalanceResponse>> {
    let address = ctx
        .ton_service
        .get_address_balance(&service_id, address)
        .await
        .map(|(a, b)| AddressBalanceDataResponse::new(a, b));

    Ok(Json(AddressBalanceResponse::from(address)))
}

pub async fn get_address_info(
    Path(address): Path<Address>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<AddressInfoResponse>> {
    let address = ctx
        .ton_service
        .get_address_info(&service_id, address)
        .await
        .map(AddressInfoDataResponse::new);

    Ok(Json(AddressInfoResponse::from(address)))
}

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
