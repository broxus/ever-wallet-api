use std::collections::HashMap;

use bigdecimal::{BigDecimal, Zero};
use dexpa::currency::Currency;
use futures::future::BoxFuture;
use futures::FutureExt;

use super::Context;
use crate::api::requests::*;
use crate::api::responses::*;
use crate::api::utils::*;
use crate::models::account_enums::{AddressResponse, TonStatus};
use crate::models::address::Address;
use crate::models::service_id::ServiceId;

pub fn post_address_create(
    service_id: ServiceId,
    input: CreateAddressRequest,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let address = ctx
            .ton_service
            .create_address(&service_id, &input.clone().into())
            .await
            .map(From::from);
        let res = AccountAddressResponse::from(address);
        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_address_check(
    service_id: ServiceId,
    input: PostAddressBalanceRequest,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let address = ctx
            .ton_service
            .check_address(&service_id, &input.address)
            .await
            .map(PostAddressValidResponse::new);
        let res = PostCheckedAddressResponse::from(address);
        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn get_address_balance(
    address: Address,
    service_id: ServiceId,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let address = ctx
            .ton_service
            .get_address_balance(&service_id, &address)
            .await
            .map(AddressResponse::from);
        let res = AccountAddressResponse::from(address);

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_transactions_create(
    service_id: ServiceId,
    input: PostTonTransactionSendRequest,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let transaction = ctx
            .ton_service
            .create_transaction(&service_id, &input.into())
            .await
            .map(From::from);
        let res = AccountTransactionResponse::from(transaction);
        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn get_transactions_mh(
    message_hash: String,
    service_id: ServiceId,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let transaction = ctx
            .ton_service
            .get_transaction_by_mh(&service_id, &message_hash)
            .await
            .map(From::from);
        let res = AccountTransactionResponse::from(transaction);

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn get_transactions_h(
    transaction_hash: String,
    service_id: ServiceId,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let transaction = ctx
            .ton_service
            .get_transaction_by_h(&service_id, &transaction_hash)
            .await
            .map(From::from);
        let res = AccountTransactionResponse::from(transaction);

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_events(
    service_id: ServiceId,
    input: PostTonTransactionEventsRequest,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let transactions_events = ctx
            .ton_service
            .search_events(&service_id, &input.event_status)
            .await?;
        let events: Vec<_> = transactions_events
            .into_iter()
            .map(AccountTransactionEventResponse::from)
            .collect();
        let res = TonEventsResponse {
            status: TonStatus::Ok,
            data: Some(EventsResponse {
                count: events.len() as i32,
                items: events,
            }),
            error_message: None,
        };

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_events_mark(
    service_id: ServiceId,
    input: PostTonMarkEventsRequest,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        ctx.ton_service.mark_event(&service_id, &input.id).await?;
        let res = MarkEventsResponse {
            status: TonStatus::Ok,
            error_message: None,
        };

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}
