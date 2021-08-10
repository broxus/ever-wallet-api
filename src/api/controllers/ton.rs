use std::collections::HashMap;

use dexpa::currency::Currency;
use futures::future::BoxFuture;
use futures::FutureExt;

use super::Context;
use crate::api::requests::*;
use crate::api::responses::*;
use crate::api::utils::*;
use crate::models::balances::BalancesSearch;
use crate::models::transactions::TransactionsSearch;
use bigdecimal::{BigDecimal, Zero};

pub fn post_transactions(
    ctx: Context,
    input: TransactionsRequest,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let input_offset = input.offset;
        let input_limit = input.limit;

        let transactions = ctx
            .tokens_service
            .search_transactions(&input.clone().into())
            .await
            .map_err(|e| warp::reject::custom(BadRequestError { 0: e.to_string() }))?;

        let transactions = transactions
            .into_iter()
            .map(TransactionInfoResponse::from)
            .collect::<Vec<_>>();

        let total_count = ctx
            .tokens_service
            .count_transactions(&TransactionsSearch::from(input.clone()))
            .await
            .map_err(|e| warp::reject::custom(BadRequestError { 0: e.to_string() }))?;

        let res = TransactionsInfoAndCountResponse::new(
            transactions,
            input_offset,
            input_limit,
            total_count,
        );
        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn get_tokens(ctx: Context) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let tokens = ctx
            .tokens_service
            .get_all_tokens()
            .await
            .map_err(|e| warp::reject::custom(BadRequestError { 0: e.to_string() }))?;
        let res = tokens
            .into_iter()
            .map(RootTokenContractResponse::from)
            .collect::<Vec<_>>();

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn get_token_owner_by_address(
    address: String,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let token = ctx
            .tokens_service
            .get_token_owner_by_address(address)
            .await
            .map_err(|e| warp::reject::custom(BadRequestError { 0: e.to_string() }))?;

        let res = TokenOwnerResponse::from(token);

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn get_root_contract_by_address(
    address: String,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let (root_token, mut total_supply) = ctx
            .tokens_service
            .get_root_token_by_address(address)
            .await
            .map_err(|e| {
                if e.to_string().contains(
                    "no rows returned by a query that expected to return at least one row",
                ) {
                    warp::reject::not_found()
                } else {
                    warp::reject::custom(BadRequestError { 0: e.to_string() })
                }
            })?;

        let mut res = RootContractInfoWithTotalSupplyResponse::from(root_token);
        res.total_supply = total_supply;

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_root_contracts_by_symbol_substring(
    input: RootTokenContractsSearchRequest,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let input_offset = input.offset;
        let input_limit = input.limit;

        let (root_tokens, total_count) = ctx
            .tokens_service
            .post_root_tokens_by_token_substring(input)
            .await
            .map_err(|e| warp::reject::custom(BadRequestError { 0: e.to_string() }))?;

        let root_token_contracts = root_tokens
            .into_iter()
            .map(|(root_token_contract, total_supply)| {
                let mut root_contract =
                    RootContractInfoWithTotalSupplyResponse::from(root_token_contract);
                root_contract.total_supply = total_supply;
                root_contract
            })
            .collect::<Vec<_>>();

        let res = RootTokenContractsSearchResponse {
            root_token_contracts,
            limit: input_limit,
            offset: input_offset,
            total_count,
        };

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn get_root_contracts_by_symbol_substring(
    token: String,
    ctx: Context,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let root_tokens = ctx
            .tokens_service
            .get_root_token_by_token_substring(token)
            .await
            .map_err(|e| warp::reject::custom(BadRequestError { 0: e.to_string() }))?;

        let res = root_tokens
            .into_iter()
            .map(|(x1, x2)| {
                let mut root_contract = RootContractInfoWithTotalSupplyResponse::from(x1);
                root_contract.total_supply = x2;
                root_contract
            })
            .filter(|x| x.total_supply > BigDecimal::zero())
            .collect::<Vec<_>>();

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_address_transactions(
    address: String,
    ctx: Context,
    input: AddressTransactionsRequest,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let input_offset = input.offset;
        let input_limit = input.limit;
        let search = (address, input).into();
        let transactions = ctx
            .tokens_service
            .search_transactions(&search)
            .await
            .map_err(|e| warp::reject::custom(BadRequestError { 0: e.to_string() }))?;

        let transactions = transactions
            .into_iter()
            .map(TransactionInfoResponse::from)
            .collect::<Vec<_>>();

        let total_count = ctx
            .tokens_service
            .count_transactions(&search)
            .await
            .map_err(|e| warp::reject::custom(BadRequestError { 0: e.to_string() }))?;

        let res = AddressTransactionsInfoResponse::new(
            transactions,
            input_offset,
            input_limit,
            total_count,
        );

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_address_balances(
    address: String,
    ctx: Context,
    input: AddressBalancesRequest,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let input_offset = input.offset;
        let input_limit = input.limit;

        let balances = ctx
            .tokens_service
            .search_balances(&(address.clone(), input.clone()).into())
            .await
            .map_err(|e| warp::reject::custom(BadRequestError { 0: e.to_string() }))?;

        let balances = balances
            .into_iter()
            .map(BalanceResponse::from)
            .collect::<Vec<_>>();

        let total_count = ctx
            .tokens_service
            .count_balances(&(address, input).into())
            .await
            .map_err(|e| warp::reject::custom(BadRequestError { 0: e.to_string() }))?;

        let res = BalancesAndCountResponse::new(balances, input_offset, input_limit, total_count);
        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}

pub fn post_balances(
    ctx: Context,
    input: BalancesRequest,
) -> BoxFuture<'static, Result<impl warp::Reply, warp::Rejection>> {
    async move {
        let input_offset = input.offset;
        let input_limit = input.limit;

        let balances = ctx
            .tokens_service
            .search_balances(&input.clone().into())
            .await
            .map_err(|e| warp::reject::custom(BadRequestError { 0: e.to_string() }))?;

        let res = balances
            .into_iter()
            .map(BalanceResponse::from)
            .collect::<Vec<_>>();

        let total_count = ctx
            .tokens_service
            .count_balances(&BalancesSearch::from(input))
            .await
            .map_err(|e| warp::reject::custom(BadRequestError { 0: e.to_string() }))?;

        let res = BalancesAndCountResponse::new(res, input_offset, input_limit, total_count);

        Ok(warp::reply::json(&(res)))
    }
    .boxed()
}
