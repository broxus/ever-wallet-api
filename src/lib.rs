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

    let (receive_transaction_tx, receive_transaction_rx) = mpsc::unbounded_channel();
    let (receive_token_transaction_tx, receive_token_transaction_rx) = mpsc::unbounded_channel();

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
        owners_cache.clone(),
        receive_transaction_tx,
        receive_token_transaction_tx,
    )
    .await?;
    let ton_api_client = Arc::new(TonClientImpl::new(ton_core.clone()));
    let ton_service = Arc::new(TonServiceImpl::new(
        sqlx_client.clone(),
        owners_cache.clone(),
        ton_api_client.clone(),
        callback_client.clone(),
        service_config.secret.clone(),
    ));
    let auth_service = Arc::new(AuthServiceImpl::new(sqlx_client.clone()));

    ton_core.start().await?;

    let accounts = sqlx_client
        .get_all_addresses()
        .await?
        .into_iter()
        .map(|item| UInt256::from_be_bytes(&hex::decode(item.hex).trust_me()))
        .collect::<Vec<UInt256>>();
    ton_core.add_account_subscription(accounts);

    log::debug!("tokens caching");
    log::debug!("Finish tokens caching");

    tokio::spawn(start_listening_receive_transactions(
        ton_service.clone(),
        receive_transaction_rx,
    ));

    tokio::spawn(start_listening_receive_token_transactions(
        ton_service.clone(),
        receive_token_transaction_rx,
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

async fn start_listening_receive_transactions(
    ton_service: Arc<TonServiceImpl>,
    mut rx: ReceiveTransactionRx,
) {
    tokio::spawn(async move {
        while let Some(transaction) = rx.recv().await {
            match transaction {
                ReceiveTransaction::Create(transaction) => {
                    match ton_service.create_receive_transaction(transaction).await {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Failed to create receive transaction: {:?}", err)
                        }
                    }
                }
                ReceiveTransaction::UpdateSent(transaction) => {
                    match ton_service
                        .update_sent_transaction(
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

async fn start_listening_receive_token_transactions(
    ton_service: Arc<TonServiceImpl>,
    mut rx: ReceiveTokenTransactionRx,
) {
    tokio::spawn(async move {
        while let Some(transaction) = rx.recv().await {
            match transaction {
                ReceiveTokenTransaction::Create(transaction) => {
                    let _transaction_db = ton_service
                        .create_receive_token_transaction(&transaction)
                        .await;
                }
                ReceiveTokenTransaction::UpdateSent(transaction) => {
                    let _transaction_db = ton_service
                        .update_sent_token_transaction(
                            transaction.message_hash,
                            transaction.account_workchain_id,
                            transaction.account_hex,
                            transaction.root_address,
                            &transaction.input,
                        )
                        .await;
                }
            }
        }

        rx.close();
        while rx.recv().await.is_some() {}
    });
}
