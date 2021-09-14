#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::inconsistent_struct_constructor)]

use std::net::SocketAddr;
use std::panic::PanicInfo;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use futures::prelude::*;
use nekoton_utils::TrustMe;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::mpsc;
use ton_types::UInt256;

use crate::api::*;
use crate::client::*;
use crate::models::*;
use crate::services::*;
use crate::settings::*;
use crate::sqlx_client::*;
use crate::ton_core::*;
use std::collections::HashMap;

#[allow(unused)]
mod api;
mod client;
mod models;
mod prelude;
mod services;
mod settings;
mod sqlx_client;
mod ton_core;
mod utils;

pub fn handle_panic(panic_info: &PanicInfo<'_>) {
    log::error!("{}", panic_info);
    std::process::exit(1);
}

pub async fn start_server() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let config = ApplicationConfig::from_env()?;

    let global_config = ton_indexer::GlobalConfig::from_file(&config.global_config)?;
    let service_config = Config::from_file(&config.service_config)?.load_env();

    init_logger(&service_config.logger_settings)?;

    std::panic::set_hook(Box::new(handle_panic));
    let _guard = sentry::init(
        sentry::ClientOptions::default().add_integration(sentry_panic::PanicIntegration::default()),
    );

    let pool = PgPoolOptions::new()
        .max_connections(service_config.db_pool_size)
        .connect(&service_config.database_url)
        .await
        .expect("fail pg pool");

    let sqlx_client = SqlxClient::new(pool);
    let callback_client = Arc::new(CallbackClientImpl::new());
    let owners_cache = OwnersCache::new(sqlx_client.clone()).await?;

    let ton_core = TonCore::new(
        service_config.ton_core,
        global_config,
        sqlx_client.clone(),
        owners_cache.clone(),
    )
    .await?;

    let token_whitelist = sqlx_client.get_token_whitelist().await?;

    let mut root_state_cache = HashMap::new();
    for root_address in &token_whitelist {
        let address = nekoton_utils::repack_address(&root_address.address).trust_me();
        let account = UInt256::from_be_bytes(&address.address().get_bytestring(0));
        let root_state = ton_core.get_contract_state(account).await?;
        root_state_cache.insert(address, root_state);
    }
    let root_state_cache = Arc::new(root_state_cache);

    let ton_api_client = Arc::new(TonClientImpl::new(
        ton_core.clone(),
        root_state_cache.clone(),
    ));
    let ton_service = Arc::new(TonServiceImpl::new(
        sqlx_client.clone(),
        owners_cache.clone(),
        ton_api_client.clone(),
        callback_client.clone(),
        service_config.secret.clone(),
    ));
    let auth_service = Arc::new(AuthServiceImpl::new(sqlx_client.clone()));

    let (caught_ton_transaction_tx, caught_ton_transaction_rx) = mpsc::unbounded_channel();
    let (caught_token_transaction_tx, caught_token_transaction_rx) = mpsc::unbounded_channel();

    ton_core
        .start(
            caught_ton_transaction_tx,
            caught_token_transaction_tx,
            root_state_cache,
        )
        .await?;

    tokio::spawn(start_listening_ton_transaction(
        ton_service.clone(),
        caught_ton_transaction_rx,
    ));

    tokio::spawn(start_listening_token_transaction(
        ton_service.clone(),
        caught_token_transaction_rx,
    ));

    log::debug!("start server");

    let server_addr: SocketAddr = service_config.server_addr.parse()?;
    tokio::spawn(http_service(server_addr, ton_service, auth_service));

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

fn init_logger(config: &serde_yaml::Value) -> Result<()> {
    let config = serde_yaml::from_value(config.clone())?;
    log4rs::config::init_raw_config(config)?;
    Ok(())
}

async fn start_listening_ton_transaction(
    ton_service: Arc<TonServiceImpl>,
    mut rx: CaughtTonTransactionRx,
) {
    tokio::spawn(async move {
        while let Some(transaction) = rx.recv().await {
            match transaction {
                CaughtTonTransaction::Create(transaction) => {
                    match ton_service.create_receive_transaction(transaction).await {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Failed to create receive transaction: {:?}", err)
                        }
                    }
                }
                CaughtTonTransaction::UpdateSent(transaction) => {
                    match ton_service
                        .upsert_sent_transaction(
                            transaction.message_hash,
                            transaction.account_workchain_id,
                            transaction.account_hex,
                            transaction.input,
                        )
                        .await
                    {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Failed to update sent transaction: {:?}", err)
                        }
                    }
                }
            }
        }

        rx.close();
        while rx.recv().await.is_some() {}
    });
}

async fn start_listening_token_transaction(
    ton_service: Arc<TonServiceImpl>,
    mut rx: CaughtTokenTransactionRx,
) {
    tokio::spawn(async move {
        while let Some(transaction) = rx.recv().await {
            if let Err(e) = ton_service.create_token_transaction(transaction).await {
                log::error!("Failed to create token transaction: {:?}", e)
            }
        }

        rx.close();
        while rx.recv().await.is_some() {}
    });
}
