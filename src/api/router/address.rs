use aide::axum::{routing::get_with, routing::post_with, ApiRouter};
use axum::Json;

use crate::api::{
    controllers,
    responses::{
        AddressBalanceResponse, AddressInfoResponse, AddressResponse, CheckedAddressResponse,
    },
    taged, ApiContext,
};

pub fn router() -> ApiRouter<ApiContext> {
    ApiRouter::new()
        .api_route_with(
            "/check",
            post_with(controllers::post_address_check, |op| {
                op.response::<200, Json<CheckedAddressResponse>>()
            }),
            taged("address"),
        )
        .api_route_with(
            "/create",
            post_with(controllers::post_address_create, |op| {
                op.response::<200, Json<AddressResponse>>()
            }),
            taged("address"),
        )
        .api_route_with(
            "/:address",
            get_with(controllers::get_address_balance, |op| {
                op.response::<200, Json<AddressBalanceResponse>>()
            }),
            taged("address"),
        )
        .api_route_with(
            "/:address/info",
            get_with(controllers::get_address_info, |op| {
                op.response::<200, Json<AddressInfoResponse>>()
            }),
            taged("address"),
        )
}
