mod controllers;
mod docs;
pub mod requests;
pub mod responses;
mod utils;

use std::net::SocketAddr;
use std::sync::Arc;

use warp::Filter;

use self::controllers::*;
use crate::api::utils::{bad_request, BadRequestError};
use crate::prelude::ServiceError;
use crate::services::{AuthService, TonService};

pub async fn http_service(
    server_http_addr: SocketAddr,
    ton_service: Arc<dyn TonService>,
    auth_service: Arc<dyn AuthService>,
) {
    let ctx = Context {
        ton_service,
        auth_service,
    };

    let api = filters::server(ctx).recover(customize_error);
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec![
            "content-type",
            "api-key",
            "x-real-ip",
            "timestamp",
            "sign",
        ])
        .allow_methods(vec!["GET", "POST", "DELETE", "OPTIONS", "PUT"]);
    let log = warp::log("warp");
    let routes = api.with(log).with(cors);
    warp::serve(routes).run(server_http_addr).await;
}

async fn customize_error(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(err) = err.find::<ServiceError>() {
        let resp: http::Response<hyper::Body> = err.into();
        Ok(resp)
    } else if let Some(err) = err.find::<BadRequestError>() {
        let resp: http::Response<hyper::Body> = bad_request(err.0.clone());
        Ok(resp)
    } else {
        Err(err)
    }
}

mod filters {
    use std::pin::Pin;
    use std::sync::Arc;

    use futures::Future;
    use http::{HeaderMap, HeaderValue};
    use warp::filters::BoxedFilter;
    use warp::{Filter, Rejection};

    use hyper::body::Bytes;

    use super::controllers::{self, Context};
    use crate::api::docs;
    use crate::api::utils::BadRequestError;
    use crate::models::*;
    use crate::services::AuthService;

    pub fn server(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::any().and(api_v4(ctx).or(healthcheck())).boxed()
    }

    pub fn healthcheck() -> BoxedFilter<(impl warp::Reply,)> {
        warp::path("healthcheck")
            .and(warp::get())
            .and_then(get_healthcheck)
            .boxed()
    }

    pub fn get_healthcheck(
    ) -> Pin<Box<dyn Future<Output = Result<impl warp::Reply, warp::Rejection>> + Send + 'static>>
    {
        Box::pin(async move { Ok(warp::reply::json(&())) })
    }

    pub fn api_v4(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path("ton")
            .and(warp::path("v3"))
            .and(
                swagger()
                    .or(post_address_check(ctx.clone()))
                    .or(post_address_create(ctx.clone()))
                    .or(get_address_balance(ctx.clone()))
                    .or(get_address_info(ctx.clone()))
                    .or(post_transactions_create(ctx.clone()))
                    .or(post_transactions_confirm(ctx.clone()))
                    .or(post_transactions(ctx.clone()))
                    .or(get_transactions_mh(ctx.clone()))
                    .or(get_transactions_h(ctx.clone()))
                    .or(get_transactions_id(ctx.clone()))
                    .or(get_events_id(ctx.clone()))
                    .or(post_events(ctx.clone()))
                    .or(post_events_mark(ctx.clone()))
                    .or(post_events_mark_all(ctx.clone()))
                    .or(get_tokens_address_balance(ctx.clone()))
                    .or(post_tokens_transactions_create(ctx.clone()))
                    .or(get_tokens_transactions_mh(ctx.clone()))
                    .or(get_tokens_transactions_id(ctx.clone()))
                    .or(post_tokens_events(ctx.clone()))
                    .or(post_tokens_events_mark(ctx)),
            )
            .boxed()
    }

    pub fn post_transactions_create(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path!("transactions" / "create")
            .and(warp::path::end())
            .and(warp::post())
            .and(auth_by_key(ctx.auth_service.clone()).untuple_one())
            .and(with_ctx(ctx))
            .and_then(controllers::post_transactions_create)
            .boxed()
    }

    pub fn post_transactions_confirm(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path!("transactions" / "confirm")
            .and(warp::path::end())
            .and(warp::post())
            .and(auth_by_key(ctx.auth_service.clone()).untuple_one())
            .and(with_ctx(ctx))
            .and_then(controllers::post_transactions_confirm)
            .boxed()
    }

    pub fn post_transactions(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path("transactions")
            .and(warp::path::end())
            .and(warp::post())
            .and(auth_by_key(ctx.auth_service.clone()).untuple_one())
            .and(with_ctx(ctx))
            .and_then(controllers::post_transactions)
            .boxed()
    }

    pub fn post_address_create(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path!("address" / "create")
            .and(warp::path::end())
            .and(warp::post())
            .and(auth_by_key(ctx.auth_service.clone()).untuple_one())
            .and(with_ctx(ctx))
            .and_then(controllers::post_address_create)
            .boxed()
    }

    pub fn post_address_check(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path!("address" / "check")
            .and(warp::path::end())
            .and(warp::post())
            .and(auth_by_key(ctx.auth_service.clone()).untuple_one())
            .and(with_ctx(ctx))
            .and_then(controllers::post_address_check)
            .boxed()
    }

    pub fn post_events(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path!("events")
            .and(warp::path::end())
            .and(warp::post())
            .and(auth_by_key(ctx.auth_service.clone()).untuple_one())
            .and(with_ctx(ctx))
            .and_then(controllers::post_events)
            .boxed()
    }

    pub fn post_events_mark(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path!("events" / "mark")
            .and(warp::path::end())
            .and(warp::post())
            .and(auth_by_key(ctx.auth_service.clone()).untuple_one())
            .and(with_ctx(ctx))
            .and_then(controllers::post_events_mark)
            .boxed()
    }

    pub fn post_events_mark_all(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path!("events" / "mark" / "all")
            .and(warp::path::end())
            .and(warp::post())
            .and(auth_by_key(ctx.auth_service.clone()).untuple_one())
            .and(with_ctx(ctx))
            .and_then(controllers::post_events_mark_all)
            .boxed()
    }

    pub fn get_address_balance(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path("address")
            .and(warp::path::param())
            .and(warp::path::end())
            .and(warp::get())
            .and(auth_by_key_get(ctx.auth_service.clone()))
            .and(with_ctx(ctx))
            .and_then(controllers::get_address_balance)
            .boxed()
    }

    pub fn get_address_info(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path("address")
            .and(warp::path::param())
            .and(warp::path("info"))
            .and(warp::path::end())
            .and(warp::get())
            .and(auth_by_key_get(ctx.auth_service.clone()))
            .and(with_ctx(ctx))
            .and_then(controllers::get_address_info)
            .boxed()
    }

    pub fn get_transactions_mh(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path("transactions")
            .and(warp::path("mh"))
            .and(warp::path::param())
            .and(warp::path::end())
            .and(warp::get())
            .and(auth_by_key_get(ctx.auth_service.clone()))
            .and(with_ctx(ctx))
            .and_then(controllers::get_transactions_mh)
            .boxed()
    }
    pub fn get_transactions_h(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path("transactions")
            .and(warp::path("h"))
            .and(warp::path::param())
            .and(warp::path::end())
            .and(warp::get())
            .and(auth_by_key_get(ctx.auth_service.clone()))
            .and(with_ctx(ctx))
            .and_then(controllers::get_transactions_h)
            .boxed()
    }

    pub fn get_transactions_id(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path("transactions")
            .and(warp::path("id"))
            .and(warp::path::param())
            .and(warp::path::end())
            .and(warp::get())
            .and(auth_by_key_get(ctx.auth_service.clone()))
            .and(with_ctx(ctx))
            .and_then(controllers::get_transactions_id)
            .boxed()
    }

    pub fn get_events_id(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path("events")
            .and(warp::path("id"))
            .and(warp::path::param())
            .and(warp::path::end())
            .and(warp::get())
            .and(auth_by_key_get(ctx.auth_service.clone()))
            .and(with_ctx(ctx))
            .and_then(controllers::get_events_id)
            .boxed()
    }

    pub fn get_tokens_address_balance(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path("tokens")
            .and(warp::path("address"))
            .and(warp::path::param())
            .and(warp::path::end())
            .and(warp::get())
            .and(auth_by_key_get(ctx.auth_service.clone()))
            .and(with_ctx(ctx))
            .and_then(controllers::get_tokens_address_balance)
            .boxed()
    }

    pub fn get_tokens_transactions_mh(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path("tokens")
            .and(warp::path("transactions"))
            .and(warp::path("mh"))
            .and(warp::path::param())
            .and(warp::path::end())
            .and(warp::get())
            .and(auth_by_key_get(ctx.auth_service.clone()))
            .and(with_ctx(ctx))
            .and_then(controllers::get_tokens_transactions_mh)
            .boxed()
    }

    pub fn get_tokens_transactions_id(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path("tokens")
            .and(warp::path("transactions"))
            .and(warp::path("id"))
            .and(warp::path::param())
            .and(warp::path::end())
            .and(warp::get())
            .and(auth_by_key_get(ctx.auth_service.clone()))
            .and(with_ctx(ctx))
            .and_then(controllers::get_tokens_transactions_id)
            .boxed()
    }

    pub fn post_tokens_transactions_create(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path!("tokens" / "transactions" / "create")
            .and(warp::path::end())
            .and(warp::post())
            .and(auth_by_key(ctx.auth_service.clone()).untuple_one())
            .and(with_ctx(ctx))
            .and_then(controllers::post_tokens_transactions_create)
            .boxed()
    }

    pub fn post_tokens_events(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path!("tokens" / "events")
            .and(warp::path::end())
            .and(warp::post())
            .and(auth_by_key(ctx.auth_service.clone()).untuple_one())
            .and(with_ctx(ctx))
            .and_then(controllers::post_tokens_events)
            .boxed()
    }

    pub fn post_tokens_events_mark(ctx: Context) -> BoxedFilter<(impl warp::Reply,)> {
        warp::path!("tokens" / "events" / "mark")
            .and(warp::path::end())
            .and(warp::post())
            .and(auth_by_key(ctx.auth_service.clone()).untuple_one())
            .and(with_ctx(ctx))
            .and_then(controllers::post_tokens_events_mark)
            .boxed()
    }

    pub fn swagger() -> BoxedFilter<(impl warp::Reply,)> {
        let docs = docs::swagger();
        warp::path!("swagger.yaml")
            .and(warp::get())
            .map(move || docs.clone())
            .boxed()
    }

    fn json_body<T>() -> impl Filter<Extract = ((String, T),), Error = warp::Rejection> + Clone
    where
        T: for<'de> serde::Deserialize<'de> + Send,
    {
        warp::body::bytes().and_then(|bytes: Bytes| async move {
            let body_s = std::str::from_utf8(&bytes)
                .map_err(|err| {
                    log::error!("error: {}", err);
                    warp::reject::custom(BadRequestError(err.to_string()))
                })?
                .to_string();

            let res = serde_json::from_str::<T>(&body_s).map_err(|err| {
                log::error!("error: {}", err);
                warp::reject::custom(BadRequestError(err.to_string()))
            })?;

            Ok::<_, Rejection>((body_s, res))
        })
    }

    #[allow(dead_code)]
    fn query<T>() -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone
    where
        T: for<'de> serde::Deserialize<'de> + Send + 'static,
    {
        warp::query()
    }

    #[allow(dead_code)]
    fn optional_query<T>() -> impl Filter<Extract = (T,), Error = std::convert::Infallible> + Clone
    where
        T: for<'de> serde::Deserialize<'de> + Default + Send + 'static,
    {
        warp::any()
            .and(warp::query().or(warp::any().map(T::default)))
            .unify()
    }

    #[allow(dead_code)]
    fn optional_param<T>(
    ) -> impl Filter<Extract = (Option<T>,), Error = std::convert::Infallible> + Clone
    where
        T: for<'de> serde::Deserialize<'de> + std::str::FromStr + Send + 'static,
    {
        warp::any()
            .and(
                warp::path::param::<T>()
                    .map(Some)
                    .or(warp::any().map(|| None)),
            )
            .unify()
    }

    #[allow(dead_code)]
    pub fn default_value<T: Default + Send + 'static>(
    ) -> impl Filter<Extract = (T,), Error = std::convert::Infallible> + Copy {
        warp::any().map(Default::default)
    }

    fn with_ctx(
        ctx: Context,
    ) -> impl Filter<Extract = (Context,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || ctx.clone())
    }

    fn auth_by_key<T>(
        auth: Arc<dyn AuthService>,
    ) -> impl Filter<Extract = ((ServiceId, T),), Error = warp::reject::Rejection> + Clone
    where
        T: for<'de> serde::Deserialize<'de> + serde::Serialize + Send + Clone + Sync,
    {
        warp::any()
            .map(move || auth.clone())
            .and(json_body().untuple_one())
            .and(warp::path::full())
            .and(warp::header::headers_cloned())
            .and_then(
                |auth_service: Arc<dyn AuthService>,
                 body_s: String,
                 body: T,
                 path: warp::path::FullPath,
                 headers: HeaderMap<HeaderValue>| {
                    async move {
                        match auth_service.authenticate(body_s, path, headers).await {
                            Ok(service_id) => Ok::<_, Rejection>((service_id, body)),
                            Err(e) => {
                                log::error!("{}", &e);
                                Err(e.into())
                            }
                        }
                    }
                },
            )
    }

    fn auth_by_key_get(
        auth: Arc<dyn AuthService>,
    ) -> impl Filter<Extract = (ServiceId,), Error = warp::reject::Rejection> + Clone {
        warp::any()
            .map(move || auth.clone())
            .and(warp::path::full())
            .and(warp::header::headers_cloned())
            .and_then(
                |auth_service: Arc<dyn AuthService>,
                 path: warp::path::FullPath,
                 headers: HeaderMap<HeaderValue>| async move {
                    match auth_service
                        .authenticate("".to_string(), path, headers)
                        .await
                    {
                        Ok(service_id) => Ok::<_, Rejection>(service_id),
                        Err(e) => {
                            log::error!("{}", &e);
                            Err(e.into())
                        }
                    }
                },
            )
    }
}
