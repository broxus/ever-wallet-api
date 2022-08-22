use axum::extract::Path;
use axum::{Extension, Json};
use uuid::Uuid;

use crate::api::controllers::*;
use crate::api::requests::*;
use crate::api::responses::*;
use crate::api::*;
use crate::models::*;

pub async fn post_events(
    Json(req): Json<TonTransactionEventsRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<TonEventsResponse>> {
    let transactions_events = ctx
        .ton_service
        .search_events(&service_id, &req.into())
        .await
        .map(|transactions_events| {
            let events: Vec<_> = transactions_events
                .into_iter()
                .map(AccountTransactionEvent::from)
                .collect();
            EventsResponse {
                count: events.len() as i32,
                items: events,
            }
        });

    Ok(Json(TonEventsResponse::from(transactions_events)))
}

pub async fn post_events_mark(
    Json(req): Json<TonMarkEventsRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<MarkEventsResponse>> {
    let transaction = ctx.ton_service.mark_event(&service_id, &req.id).await;

    Ok(Json(MarkEventsResponse::from(transaction)))
}

pub async fn post_events_mark_all(
    Json(req): Json<MarkAllTransactionEventRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<MarkEventsResponse>> {
    let transactions = ctx
        .ton_service
        .mark_all_events(&service_id, req.event_status)
        .await;

    Ok(Json(MarkEventsResponse::from(transactions)))
}

pub async fn post_tokens_events(
    Json(req): Json<TonTokenTransactionEventsRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<TonTokenEventsResponse>> {
    let transactions_events = ctx
        .ton_service
        .search_token_events(&service_id, &req.into())
        .await?;
    let events: Vec<_> = transactions_events
        .into_iter()
        .map(AccountTransactionEvent::from)
        .collect();
    let res = TonTokenEventsResponse {
        status: TonStatus::Ok,
        data: Some(TokenEventsResponse {
            count: events.len() as i32,
            items: events,
        }),
        error_message: None,
    };

    Ok(Json(res))
}

pub async fn post_tokens_events_mark(
    Json(req): Json<TonTokenMarkEventsRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<MarkTokenEventsResponse>> {
    let transaction = ctx.ton_service.mark_token_event(&service_id, &req.id).await;

    Ok(Json(MarkTokenEventsResponse::from(transaction)))
}

pub async fn get_events_id(
    Path(id): Path<Uuid>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<TransactionEventResponse>> {
    let event = ctx
        .ton_service
        .get_event_by_id(&service_id, &id)
        .await
        .map(From::from);

    Ok(Json(TransactionEventResponse::from(event)))
}
