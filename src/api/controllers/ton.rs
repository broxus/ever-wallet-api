use futures::future::BoxFuture;
use futures::FutureExt;

use super::Context;
use crate::api::requests::*;
use crate::api::responses::*;
use crate::models::*;

pub fn post_address_create(
    service_id: ServiceId,
    input: CreateAddressRequest,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let address = ctx
            .ton_service
            .create_address(&service_id, input.into())
            .await
            .map(From::from);
        let res = AccountAddressResponse::from(address);
        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_address_check(
    _service_id: ServiceId,
    input: PostAddressBalanceRequest,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let address = ctx
            .ton_service
            .check_address(input.address)
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
            .get_address_balance(&service_id, address)
            .await
            .map(|(a, b)| PostAddressBalanceDataResponse::new(a, b));
        let res = AddressBalanceResponse::from(address);

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn get_address_info(
    address: Address,
    service_id: ServiceId,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let address = ctx
            .ton_service
            .get_address_info(&service_id, address)
            .await
            .map(PostAddressInfoDataResponse::new);
        let res = AddressInfoResponse::from(address);

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
            .create_send_transaction(&service_id, input.into())
            .await
            .map(From::from);
        let res = AccountTransactionResponse::from(transaction);
        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_transactions_confirm(
    service_id: ServiceId,
    input: PostTonTransactionConfirmRequest,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let transaction = ctx
            .ton_service
            .create_confirm_transaction(&service_id, input.into())
            .await
            .map(From::from);
        let res = AccountTransactionResponse::from(transaction);
        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_transactions(
    service_id: ServiceId,
    input: PostTonTransactionsRequest,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let transactions = ctx
            .ton_service
            .search_transaction(&service_id, &input.into())
            .await
            .map(|transactions| {
                let transactions: Vec<_> = transactions
                    .into_iter()
                    .map(AccountTransactionDataResponse::from)
                    .collect();
                TransactionsResponse {
                    count: transactions.len() as i32,
                    items: transactions,
                }
            });

        let res = TonTransactionsResponse::from(transactions);

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

pub fn get_transactions_id(
    id: uuid::Uuid,
    service_id: ServiceId,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let transaction = ctx
            .ton_service
            .get_transaction_by_id(&service_id, &id)
            .await
            .map(From::from);
        let res = AccountTransactionResponse::from(transaction);

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn get_events_id(
    id: uuid::Uuid,
    service_id: ServiceId,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let event = ctx
            .ton_service
            .get_event_by_id(&service_id, &id)
            .await
            .map(From::from);
        let res = AccountTransactionEventResponse::from(event);

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
            .search_events(&service_id, &input.into())
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

        let res = TonEventsResponse::from(transactions_events);

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
        let transaction = ctx.ton_service.mark_event(&service_id, &input.id).await;
        let res = MarkEventsResponse::from(transaction);
        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_events_mark_all(
    service_id: ServiceId,
    input: MarkAllTransactionEventRequest,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let transactions = ctx
            .ton_service
            .mark_all_events(&service_id, input.event_status)
            .await;
        let res = MarkEventsResponse::from(transactions);
        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn get_tokens_transactions_mh(
    message_hash: String,
    service_id: ServiceId,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let transaction = ctx
            .ton_service
            .get_tokens_transaction_by_mh(&service_id, &message_hash)
            .await
            .map(From::from);
        let res = AccountTokenTransactionResponse::from(transaction);

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn get_tokens_transactions_id(
    id: uuid::Uuid,
    service_id: ServiceId,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let transaction = ctx
            .ton_service
            .get_tokens_transaction_by_id(&service_id, &id)
            .await
            .map(From::from);
        let res = AccountTokenTransactionResponse::from(transaction);

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_tokens_events(
    service_id: ServiceId,
    input: PostTonTokenTransactionEventsRequest,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let transactions_events = ctx
            .ton_service
            .search_token_events(&service_id, &input.into())
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

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_tokens_events_mark(
    service_id: ServiceId,
    input: PostTonTokenMarkEventsRequest,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        ctx.ton_service
            .mark_token_event(&service_id, &input.id)
            .await?;
        let res = MarkTokenEventsResponse {
            status: TonStatus::Ok,
            error_message: None,
        };

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn get_tokens_address_balance(
    address: Address,
    service_id: ServiceId,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let addresses = ctx
            .ton_service
            .get_token_address_balance(&service_id, &address)
            .await
            .map(|a| {
                a.into_iter()
                    .map(|(a, b)| TokenBalanceResponse::new(a, b))
                    .collect::<Vec<TokenBalanceResponse>>()
            });
        let res = AccountTokenBalanceResponse::from(addresses);

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_tokens_transactions_create(
    service_id: ServiceId,
    input: PostTonTokenTransactionSendRequest,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let transaction = ctx
            .ton_service
            .create_send_token_transaction(&service_id, &input.into())
            .await
            .map(From::from);
        let res = AccountTransactionResponse::from(transaction);
        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_tokens_transactions_burn(
    service_id: ServiceId,
    input: PostTonTokenTransactionBurnRequest,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let transaction = ctx
            .ton_service
            .create_burn_token_transaction(&service_id, &input.into())
            .await
            .map(From::from);
        let res = AccountTransactionResponse::from(transaction);
        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_tokens_transactions_mint(
    service_id: ServiceId,
    input: PostTonTokenTransactionMintRequest,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let transaction = ctx
            .ton_service
            .create_mint_token_transaction(&service_id, &input.into())
            .await
            .map(From::from);
        let res = AccountTransactionResponse::from(transaction);
        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn get_metrics(ctx: Context) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let metrics = ctx.ton_service.get_metrics().await?;
        let res = MetricsResponse::from(metrics);

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}
