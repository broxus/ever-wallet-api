#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::inconsistent_struct_constructor)]

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use dexpa::errors::*;
use dexpa::utils::handle_panic;
use futures::prelude::*;
use r2d2_redis::RedisConnectionManager;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;

use crate::api::http_service;
use crate::indexer::TonIndexer;
use crate::models::owners_cache::OwnersCache;
use crate::services::{AuthServiceImpl, TonServiceImpl};
use crate::settings::{Config, ConfigExt};
use crate::sqlx_client::SqlxClient;

#[allow(unused)]
mod api;
mod indexer;
mod models;
mod prelude;
mod redis;
mod services;
mod settings;
mod sqlx_client;

pub async fn start_server() -> StdResult<()> {
    let app_config = ApplicationConfig::from_env()?;

    let config = Config::from_file(&app_config.service_config)?;
    let global_config = ton_indexer::GlobalConfig::from_file(&app_config.global_config)?;

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
    let owners_hash = OwnersCache::new(sqlx_client.clone()).await?;
    let ton_service = Arc::new(TonServiceImpl::new(
        sqlx_client.clone(),
        owners_hash.clone(),
    ));
    let auth_service = Arc::new(AuthServiceImpl::new(
        sqlx_client.clone(),
        redis_pool.clone(),
    ));
    let sqlx_client_clone = sqlx_client.clone();
    log::debug!("tokens caching");
    log::debug!("Finish tokens caching");

    log::debug!("start server");

    tokio::spawn(http_service(config.server_addr, ton_service, auth_service));

    tokio::spawn(dexpa::net::healthcheck_service(config.healthcheck_addr));

    let engine = TonIndexer::new(config.indexer.clone(), global_config).await?;
    engine.start().await?;

    future::pending().await
}

#[derive(Deserialize)]
struct ApplicationConfig {
    service_config: PathBuf,
    global_config: PathBuf,
}

impl ApplicationConfig {
    fn from_env() -> Result<Self> {
        let mut config = config::Config::new();
        config.merge(config::Environment::new())?;
        let config: Self = config.try_into()?;
        Ok(config)
    }
}
