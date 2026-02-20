use aide::axum::{routing::get_with, routing::post_with, ApiRouter};
use axum::Json;

use crate::api::{
    controllers,
    responses::{
        CheckedAddressResponse, MarkTokenEventsResponse, TokenBalanceResponse,
        TokenTransactionResponse, TokenWhitelistResponse, TonTokenEventsResponse,
        TransactionResponse,
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
            taged("tokens"),
        )
        .api_route_with(
            "/address/{address}",
            get_with(controllers::get_token_address_balance, |op| {
                op.response::<200, Json<TokenBalanceResponse>>()
            }),
            taged("tokens"),
        )
        .api_route_with(
            "/transactions/id/{internal_id}",
            get_with(controllers::get_tokens_transactions_id, |op| {
                op.response::<200, Json<TokenTransactionResponse>>()
            }),
            taged("tokens"),
        )
        .api_route_with(
            "/transactions/mh/{message_hash}",
            get_with(controllers::get_tokens_transactions_mh, |op| {
                op.response::<200, Json<TokenTransactionResponse>>()
            }),
            taged("tokens"),
        )
        .api_route_with(
            "/transactions/create",
            post_with(controllers::post_tokens_transactions_create, |op| {
                op.response::<200, Json<TransactionResponse>>()
            }),
            taged("tokens"),
        )
        .api_route_with(
            "/transactions/burn",
            post_with(controllers::post_tokens_transactions_burn, |op| {
                op.response::<200, Json<TransactionResponse>>()
            }),
            taged("tokens"),
        )
        .api_route_with(
            "/transactions/mint",
            post_with(controllers::post_tokens_transactions_mint, |op| {
                op.response::<200, Json<TransactionResponse>>()
            }),
            taged("tokens"),
        )
        .api_route_with(
            "/events",
            post_with(controllers::post_tokens_events, |op| {
                op.response::<200, Json<TonTokenEventsResponse>>()
            }),
            taged("tokens"),
        )
        .api_route_with(
            "/events/mark",
            post_with(controllers::post_tokens_events_mark, |op| {
                op.response::<200, Json<MarkTokenEventsResponse>>()
            }),
            taged("tokens"),
        )
        .api_route_with(
            "/whitelist",
            get_with(controllers::get_token_whitelist, |op| {
                op.response::<200, Json<TokenWhitelistResponse>>()
            }),
            taged("tokens"),
        )
}
