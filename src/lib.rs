#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::inconsistent_struct_constructor)]

use std::sync::Arc;

use dexpa::errors::*;
use dexpa::utils::handle_panic;
use futures::prelude::*;
use r2d2_redis::RedisConnectionManager;
use sqlx::postgres::PgPoolOptions;

use crate::api::http_service;
use crate::models::owners_cache::OwnersCache;
use crate::models::root_contracts_cache::RootContractsCache;
use crate::services::{AuthServiceImpl, TonServiceImpl};
use crate::settings::Config;
use crate::sqlx_client::SqlxClient;
use crate::ws_indexer::ton_indexer_stream;

#[allow(unused)]
mod api;
mod models;
mod prelude;
mod redis;
mod services;
mod settings;
mod sqlx_client;
pub mod ws_indexer;

pub async fn start_server() -> StdResult<()> {
    let config = get_config();
    // Prepare logger
    stackdriver_logger::init_with_cargo!();

    std::panic::set_hook(Box::new(handle_panic));
    let _guard = sentry::init(
        sentry::ClientOptions::default().add_integration(sentry_panic::PanicIntegration::default()),
    );

    let pool = PgPoolOptions::new()
        .max_connections(config.db_pool_size)
        .connect(&config.database_url)
        .await
        .expect("fail pg pool");

    let redis_manager = RedisConnectionManager::new(config.redis_addr.as_str())
        .expect("Can not create redis manager");
    let redis_pool = r2d2::Pool::builder()
        .build(redis_manager)
        .expect("Can not connect to redis");

    let config = Arc::new(config);
    let sqlx_client = SqlxClient::new(pool);
    let tokens_service = Arc::new(TonServiceImpl::new(sqlx_client.clone()));
    let auth_service = Arc::new(AuthServiceImpl::new(
        sqlx_client.clone(),
        redis_pool.clone(),
    ));
    let sqlx_client_clone = sqlx_client.clone();
    log::debug!("tokens caching");
    let owners_hash = OwnersCache::new(sqlx_client.clone()).await?;
    let contracts_hash = RootContractsCache::new(sqlx_client).await?;
    log::debug!("Finish tokens caching");

    tokio::spawn(ton_indexer_stream(
        sqlx_client_clone,
        owners_hash.clone(),
        contracts_hash,
        redis_pool,
    ));

    log::debug!("start server");

    tokio::spawn(http_service(
        config.server_addr,
        tokens_service,
        auth_service,
        owners_hash,
    ));

    tokio::spawn(dexpa::net::healthcheck_service(config.healthcheck_addr));

    future::pending().await
}

fn get_config() -> Config {
    settings::Config::new().unwrap_or_else(|e| panic!("Error parsing config: {}", e))
}
