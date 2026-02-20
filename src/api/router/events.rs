use aide::axum::{routing::get_with, routing::post_with, ApiRouter};
use axum::Json;

use crate::api::{
    controllers,
    responses::{MarkEventsResponse, TonEventsResponse, TransactionEventResponse},
    taged, ApiContext,
};

pub fn router() -> ApiRouter<ApiContext> {
    ApiRouter::new()
        .api_route_with(
            "/",
            post_with(controllers::post_events, |op| {
                op.response::<200, Json<TonEventsResponse>>()
            }),
            taged("events"),
        )
        .api_route_with(
            "/mark",
            post_with(controllers::post_events_mark, |op| {
                op.response::<200, Json<MarkEventsResponse>>()
            }),
            taged("events"),
        )
        .api_route_with(
            "/mark/all",
            post_with(controllers::post_events_mark_all, |op| {
                op.response::<200, Json<MarkEventsResponse>>()
            }),
            taged("events"),
        )
        .api_route_with(
            "/id/{id}",
            get_with(controllers::get_events_id, |op| {
                op.response::<200, Json<TransactionEventResponse>>()
            }),
            taged("events"),
        )
}
