use aide::axum::{routing::get_with, routing::post_with, ApiRouter};
use axum::Json;

use crate::api::{
    controllers,
    responses::{TonTransactionsResponse, TransactionResponse},
    taged, ApiContext,
};

pub fn router() -> ApiRouter<ApiContext> {
    ApiRouter::new()
        .api_route_with(
            "/",
            post_with(controllers::post_transactions, |op| {
                op.response::<200, Json<TonTransactionsResponse>>()
            }),
            taged("transactions"),
        )
        .api_route_with(
            "/create",
            post_with(controllers::post_transactions_create, |op| {
                op.response::<200, Json<TransactionResponse>>()
            }),
            taged("transactions"),
        )
        .api_route_with(
            "/confirm",
            post_with(controllers::post_transactions_confirm, |op| {
                op.response::<200, Json<TransactionResponse>>()
            }),
            taged("transactions"),
        )
        .api_route_with(
            "/id/{id}",
            get_with(controllers::get_transactions_id, |op| {
                op.response::<200, Json<TransactionResponse>>()
            }),
            taged("transactions"),
        )
        .api_route_with(
            "/h/{hash}",
            get_with(controllers::get_transactions_h, |op| {
                op.response::<200, Json<TransactionResponse>>()
            }),
            taged("transactions"),
        )
        .api_route_with(
            "/mh/{message_hash}",
            get_with(controllers::get_transactions_mh, |op| {
                op.response::<200, Json<TransactionResponse>>()
            }),
            taged("transactions"),
        )
}
