use aide::axum::{routing::post_with, ApiRouter};
use axum::Json;

use crate::api::{
    controllers,
    responses::{
        EncodedCellResponse, ReadContractResponse, SetCallbackResponse, SignedMessageHashResponse,
        TransactionResponse, UnsignedMessageHashResponse,
    },
    taged, ApiContext,
};

pub fn router() -> ApiRouter<ApiContext> {
    ApiRouter::new()
        .api_route_with(
            "/callback",
            post_with(controllers::post_set_callback, |op| {
                op.response::<200, Json<SetCallbackResponse>>()
            }),
            taged("misc"),
        )
        .api_route_with(
            "/read-contract",
            post_with(controllers::post_read_contract, |op| {
                op.response::<200, Json<ReadContractResponse>>()
            }),
            taged("misc"),
        )
        .api_route_with(
            "/encode-into-cell",
            post_with(controllers::post_encode_tvm_cell, |op| {
                op.response::<200, Json<EncodedCellResponse>>()
            }),
            taged("misc"),
        )
        .api_route_with(
            "/prepare-message",
            post_with(controllers::post_prepare_generic_message, |op| {
                op.response::<200, Json<UnsignedMessageHashResponse>>()
            }),
            taged("misc"),
        )
        .api_route_with(
            "/send-signed-message",
            post_with(controllers::post_send_signed_message, |op| {
                op.response::<200, Json<SignedMessageHashResponse>>()
            }),
            taged("misc"),
        )
        .api_route_with(
            "/send-message",
            post_with(controllers::post_send_generic_message, |op| {
                op.response::<200, Json<TransactionResponse>>()
            }),
            taged("misc"),
        )
}
