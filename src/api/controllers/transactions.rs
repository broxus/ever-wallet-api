use axum::extract::Path;
use axum::{Extension, Json};
use metrics::{histogram, increment_counter};
use tokio::time::Instant;
use uuid::Uuid;

use crate::api::controllers::*;
use crate::api::requests::*;
use crate::api::responses::*;
use crate::api::*;

pub async fn post_transactions(
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
    Json(req): Json<TonTransactionsRequest>,
) -> Result<Json<TonTransactionsResponse>> {
    let transactions = ctx
        .ton_service
        .search_transaction(&service_id, &req.into())
        .await
        .map(|transactions| {
            let transactions: Vec<_> = transactions
                .into_iter()
                .map(TransactionDataResponse::from)
                .collect();
            TransactionsResponse {
                count: transactions.len() as i32,
                items: transactions,
            }
        });

    Ok(Json(TonTransactionsResponse::from(transactions)))
}

pub async fn post_transactions_create(
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
    Json(req): Json<TonTransactionSendRequest>,
) -> Result<Json<TransactionResponse>> {
    let start = Instant::now();

    let transaction = ctx
        .ton_service
        .create_send_transaction(&service_id, req.into())
        .await
        .map(From::from);

    let elapsed = start.elapsed();
    histogram!("execution_time_seconds", elapsed, "method" => "transactionCreate");
    increment_counter!("requests_processed", "method" => "transactionCreate");

    Ok(Json(TransactionResponse::from(transaction)))
}

pub async fn post_transactions_confirm(
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
    Json(req): Json<TonTransactionConfirmRequest>,
) -> Result<Json<TransactionResponse>> {
    let start = Instant::now();

    let transaction = ctx
        .ton_service
        .create_confirm_transaction(&service_id, req.into())
        .await
        .map(From::from);

    let elapsed = start.elapsed();
    histogram!("execution_time_seconds", elapsed, "method" => "transactionConfirm");
    increment_counter!("requests_processed", "method" => "transactionConfirm");

    Ok(Json(TransactionResponse::from(transaction)))
}

pub async fn get_transactions_mh(
    Path(message_hash): Path<String>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<TransactionResponse>> {
    let transaction = ctx
        .ton_service
        .get_transaction_by_mh(&service_id, &message_hash)
        .await
        .map(From::from);

    Ok(Json(TransactionResponse::from(transaction)))
}

pub async fn get_transactions_h(
    Path(hash): Path<String>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<TransactionResponse>> {
    let transaction = ctx
        .ton_service
        .get_transaction_by_h(&service_id, &hash)
        .await
        .map(From::from);

    Ok(Json(TransactionResponse::from(transaction)))
}

pub async fn get_transactions_id(
    Path(id): Path<Uuid>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<TransactionResponse>> {
    let transaction = ctx
        .ton_service
        .get_transaction_by_id(&service_id, &id)
        .await
        .map(From::from);

    Ok(Json(TransactionResponse::from(transaction)))
}

pub async fn get_tokens_transactions_id(
    Path(internal_id): Path<Uuid>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<TokenTransactionResponse>> {
    let transaction = ctx
        .ton_service
        .get_tokens_transaction_by_id(&service_id, &internal_id)
        .await
        .map(From::from);

    Ok(Json(TokenTransactionResponse::from(transaction)))
}

pub async fn get_tokens_transactions_mh(
    Path(message_hash): Path<String>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<TokenTransactionResponse>> {
    let transaction = ctx
        .ton_service
        .get_tokens_transaction_by_mh(&service_id, &message_hash)
        .await
        .map(From::from);

    Ok(Json(TokenTransactionResponse::from(transaction)))
}

pub async fn post_tokens_transactions_create(
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
    Json(req): Json<TonTokenTransactionSendRequest>,
) -> Result<Json<TransactionResponse>> {
    let start = Instant::now();

    let transaction = ctx
        .ton_service
        .create_send_token_transaction(&service_id, &req.into())
        .await
        .map(From::from);

    let elapsed = start.elapsed();
    histogram!("execution_time_seconds", elapsed, "method" => "tokenTransactionCreate");
    increment_counter!("requests_processed", "method" => "tokenTransactionCreate");

    Ok(Json(TransactionResponse::from(transaction)))
}

pub async fn post_tokens_transactions_burn(
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
    Json(req): Json<TonTokenTransactionBurnRequest>,
) -> Result<Json<TransactionResponse>> {
    let start = Instant::now();

    let transaction = ctx
        .ton_service
        .create_burn_token_transaction(&service_id, &req.into())
        .await
        .map(From::from);

    let elapsed = start.elapsed();
    histogram!("execution_time_seconds", elapsed, "method" => "tokenTransactionBurn");
    increment_counter!("requests_processed", "method" => "tokenTransactionBurn");

    Ok(Json(TransactionResponse::from(transaction)))
}

pub async fn post_tokens_transactions_mint(
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
    Json(req): Json<TonTokenTransactionMintRequest>,
) -> Result<Json<TransactionResponse>> {
    let start = Instant::now();

    let transaction = ctx
        .ton_service
        .create_mint_token_transaction(&service_id, &req.into())
        .await
        .map(From::from);

    let elapsed = start.elapsed();
    histogram!("execution_time_seconds", elapsed, "method" => "tokenTransactionMint");
    increment_counter!("requests_processed", "method" => "tokenTransactionMint");

    Ok(Json(TransactionResponse::from(transaction)))
}
