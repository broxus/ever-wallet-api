use std::net::SocketAddr;
use std::panic::PanicInfo;
use std::sync::Arc;

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::mpsc;

use crate::api::*;
use crate::client::*;
use crate::models::*;
use crate::services::*;
use crate::settings::*;
use crate::sqlx_client::*;
use crate::ton_core::*;
use crate::utils::*;

pub async fn server_run(config: AppConfig, global_config: ton_indexer::GlobalConfig) -> Result<()> {
    std::panic::set_hook(Box::new(handle_panic));
    let _guard = sentry::init(
        sentry::ClientOptions::default().add_integration(sentry_panic::PanicIntegration::default()),
    );

    let pool = PgPoolOptions::new()
        .max_connections(config.db_pool_size)
        .connect(&config.database_url)
        .await
        .expect("fail pg pool");

    let sqlx_client = SqlxClient::new(pool);
    let callback_client = Arc::new(CallbackClientImpl::new());
    let owners_cache = OwnersCache::new(sqlx_client.clone()).await?;

    let (caught_ton_transaction_tx, caught_ton_transaction_rx) = mpsc::unbounded_channel();
    let (caught_token_transaction_tx, caught_token_transaction_rx) = mpsc::unbounded_channel();

    let ton_core = TonCore::new(
        config.ton_core,
        global_config,
        sqlx_client.clone(),
        owners_cache.clone(),
        caught_ton_transaction_tx,
        caught_token_transaction_tx,
    )
    .await?;

    let ton_api_client = Arc::new(TonClientImpl::new(ton_core.clone(), sqlx_client.clone()));
    ton_api_client.start().await?;

    let ton_service = Arc::new(TonServiceImpl::new(
        sqlx_client.clone(),
        owners_cache.clone(),
        ton_api_client.clone(),
        callback_client.clone(),
        config.key.clone(),
    ));
    ton_service.start().await?;

    let auth_service = Arc::new(AuthServiceImpl::new(sqlx_client.clone()));

    tokio::spawn(start_listening_ton_transaction(
        ton_service.clone(),
        caught_ton_transaction_rx,
    ));

    tokio::spawn(start_listening_token_transaction(
        ton_service.clone(),
        caught_token_transaction_rx,
    ));

    ton_core.start().await?;

    let server_addr: SocketAddr = config.server_addr.parse()?;
    tokio::spawn(http_service(server_addr, ton_service, auth_service));

    Ok(())
}

fn handle_panic(panic_info: &PanicInfo<'_>) {
    log::error!("{}", panic_info);
    std::process::exit(1);
}

async fn start_listening_ton_transaction(
    ton_service: Arc<TonServiceImpl>,
    mut rx: CaughtTonTransactionRx,
) {
    tokio::spawn(async move {
        while let Some((transaction, state)) = rx.recv().await {
            match transaction {
                CaughtTonTransaction::Create(transaction) => {
                    match ton_service.create_receive_transaction(transaction).await {
                        Ok(_) => {
                            state.send(HandleTransactionStatus::Success).ok();
                        }
                        Err(err) => {
                            state.send(HandleTransactionStatus::Fail).ok();
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
                        Ok(_) => {
                            state.send(HandleTransactionStatus::Success).ok();
                        }
                        Err(err) => {
                            state.send(HandleTransactionStatus::Fail).ok();
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
        while let Some((transaction, state)) = rx.recv().await {
            match ton_service.create_token_transaction(transaction).await {
                Ok(_) => {
                    state.send(HandleTransactionStatus::Success).ok();
                }
                Err(e) => {
                    state.send(HandleTransactionStatus::Fail).ok();
                    log::error!("Failed to create token transaction: {:?}", e)
                }
            };
        }

        rx.close();
        while rx.recv().await.is_some() {}
    });
}
