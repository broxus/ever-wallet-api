use std::sync::Arc;

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::mpsc;

use crate::api::*;
use crate::client::*;
use crate::metrics_exporter::*;
use crate::models::*;
use crate::services::*;
use crate::settings::*;
use crate::sqlx_client::*;
use crate::ton_core::*;
use crate::utils::*;

pub struct Engine {
    metrics_exporter: Arc<MetricsExporter>,
    context: Arc<EngineContext>,
}

impl Engine {
    pub async fn new(
        config: AppConfig,
        global_config: ton_indexer::GlobalConfig,
        shutdown_requests_tx: ShutdownRequestsTx,
    ) -> Result<Arc<Self>> {
        let metrics_exporter =
            MetricsExporter::with_config(config.metrics_settings.clone()).await?;

        let context = EngineContext::new(config, global_config, shutdown_requests_tx).await?;

        Ok(Arc::new(Self {
            metrics_exporter,
            context,
        }))
    }

    pub async fn start(self: &Arc<Self>) -> Result<()> {
        self.start_metrics_exporter();

        self.context.start().await?;

        tokio::spawn(http_service(
            self.context.config.server_addr,
            self.context.ton_service.clone(),
            self.context.auth_service.clone(),
            self.context.memory_storage.clone(),
        ));

        // Done
        Ok(())
    }

    fn start_metrics_exporter(self: &Arc<Self>) {
        let engine = Arc::downgrade(self);
        let handle = Arc::downgrade(self.metrics_exporter.handle());

        tokio::spawn(async move {
            loop {
                let handle = match (engine.upgrade(), handle.upgrade()) {
                    // Update next metrics buffer
                    (Some(engine), Some(handle)) => {
                        let mut buffer = handle.buffers().acquire_buffer().await;
                        buffer.write(LabeledTonServiceMetrics(&engine.context));
                        buffer.write(LabeledTonSubscriberMetrics(&engine.context));

                        drop(buffer);
                        handle
                    }
                    // Engine is already dropped
                    _ => return,
                };

                handle.wait().await;
            }
        });
    }
}

pub struct EngineContext {
    pub shutdown_requests_tx: ShutdownRequestsTx,
    pub ton_core: Arc<TonCore>,
    pub ton_client: Arc<TonClientImpl>,
    pub ton_service: Arc<TonServiceImpl>,
    pub auth_service: Arc<AuthServiceImpl>,
    pub memory_storage: Arc<StorageHandler>,
    pub config: AppConfig,
}

impl EngineContext {
    async fn new(
        config: AppConfig,
        global_config: ton_indexer::GlobalConfig,
        shutdown_requests_tx: ShutdownRequestsTx,
    ) -> Result<Arc<Self>> {
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
            config.ton_core.clone(),
            global_config,
            sqlx_client.clone(),
            owners_cache,
            caught_ton_transaction_tx,
            caught_token_transaction_tx,
        )
        .await?;

        let ton_client = Arc::new(TonClientImpl::new(ton_core.clone(), sqlx_client.clone()));

        let ton_service = Arc::new(TonServiceImpl::new(
            sqlx_client.clone(),
            ton_client.clone(),
            callback_client.clone(),
            config.key.clone(),
        ));

        let auth_service = Arc::new(AuthServiceImpl::new(sqlx_client.clone()));

        let memory_storage = Arc::new(StorageHandler::default());

        let engine_context = Arc::new(Self {
            shutdown_requests_tx,
            ton_core,
            ton_client,
            ton_service,
            auth_service,
            memory_storage,
            config,
        });

        engine_context.start_listening_ton_transaction(caught_ton_transaction_rx);
        engine_context.start_listening_token_transaction(caught_token_transaction_rx);

        Ok(engine_context)
    }

    async fn start(&self) -> Result<()> {
        self.ton_client.start().await?;
        self.ton_service.start().await?;
        self.ton_core.start().await?;

        Ok(())
    }

    fn start_listening_ton_transaction(self: &Arc<Self>, mut rx: CaughtTonTransactionRx) {
        let engine_context = Arc::downgrade(self);

        tokio::spawn(async move {
            while let Some((transaction, state)) = rx.recv().await {
                let engine_context = match engine_context.upgrade() {
                    Some(engine_context) => engine_context,
                    None => {
                        log::error!("Engine is already dropped");
                        return;
                    }
                };

                match transaction {
                    CaughtTonTransaction::Create(transaction) => {
                        match engine_context
                            .ton_service
                            .create_receive_transaction(transaction)
                            .await
                        {
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
                        if let Err(err) = engine_context
                            .ton_service
                            .update_token_transaction(
                                transaction.message_hash.clone(),
                                transaction.account_workchain_id,
                                transaction.account_hex.clone(),
                                transaction.input.messages_hash.clone(),
                            )
                            .await
                        {
                            log::error!("Failed to update token transaction: {:?}", err)
                        }

                        match engine_context
                            .ton_service
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

    fn start_listening_token_transaction(self: &Arc<Self>, mut rx: CaughtTokenTransactionRx) {
        let engine_context = Arc::downgrade(self);

        tokio::spawn(async move {
            while let Some((transaction, state)) = rx.recv().await {
                let engine_context = match engine_context.upgrade() {
                    Some(engine_context) => engine_context,
                    None => {
                        log::error!("Engine is already dropped");
                        return;
                    }
                };

                match engine_context
                    .ton_service
                    .create_receive_token_transaction(transaction)
                    .await
                {
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
}

struct LabeledTonServiceMetrics<'a>(&'a EngineContext);

impl std::fmt::Display for LabeledTonServiceMetrics<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let metrics = self.0.ton_service.metrics();

        f.begin_metric("ton_service_create_address_total_requests")
            .value(metrics.create_address_count)?;

        f.begin_metric("ton_service_send_transaction_total_requests")
            .value(metrics.send_transaction_count)?;

        f.begin_metric("ton_service_recv_transaction_total_requests")
            .value(metrics.recv_transaction_count)?;

        f.begin_metric("ton_service_send_token_transaction_total_requests")
            .value(metrics.send_token_transaction_count)?;

        f.begin_metric("ton_service_recv_token_transaction_total_requests")
            .value(metrics.recv_token_transaction_count)?;

        Ok(())
    }
}

struct LabeledTonSubscriberMetrics<'a>(&'a EngineContext);

impl std::fmt::Display for LabeledTonSubscriberMetrics<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::sync::atomic::Ordering;

        let metrics = self.0.ton_core.context.ton_subscriber.metrics();
        let indexer_metrics = self.0.ton_core.context.ton_engine.metrics();

        f.begin_metric("ton_subscriber_ready")
            .value(metrics.ready as u8)?;

        if metrics.current_utime > 0 {
            let mc_time_diff = indexer_metrics.mc_time_diff.load(Ordering::Acquire);
            let shard_client_time_diff = indexer_metrics
                .shard_client_time_diff
                .load(Ordering::Acquire);

            let last_mc_block_seqno = indexer_metrics.last_mc_block_seqno.load(Ordering::Acquire);
            let last_shard_client_mc_block_seqno = indexer_metrics
                .last_shard_client_mc_block_seqno
                .load(Ordering::Acquire);

            f.begin_metric("ton_subscriber_current_utime")
                .value(metrics.current_utime)?;

            f.begin_metric("ton_subscriber_time_diff")
                .value(mc_time_diff)?;

            f.begin_metric("ton_subscriber_shard_client_time_diff")
                .value(shard_client_time_diff)?;

            f.begin_metric("ton_subscriber_mc_block_seqno")
                .value(last_mc_block_seqno)?;

            f.begin_metric("ton_subscriber_shard_client_mc_block_seqno")
                .value(last_shard_client_mc_block_seqno)?;
        }

        f.begin_metric("ton_subscriber_pending_message_count")
            .value(metrics.pending_message_count)?;

        Ok(())
    }
}

pub type ShutdownRequestsRx = mpsc::UnboundedReceiver<()>;
pub type ShutdownRequestsTx = mpsc::UnboundedSender<()>;
