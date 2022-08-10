use std::sync::Arc;

use anyhow::Result;
use everscale_network::utils::FxDashMap;
use pomfrit::formatter::*;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use crate::api::*;
use crate::client::*;
use crate::models::*;
use crate::services::*;
use crate::settings::*;
use crate::sqlx_client::*;
use crate::ton_core::*;
use crate::utils::*;

pub struct Engine {
    context: Arc<EngineContext>,
    _metrics_exporter: Arc<pomfrit::MetricsExporter>,
}

impl Engine {
    pub async fn new(
        config: AppConfig,
        global_config: ton_indexer::GlobalConfig,
        shutdown_requests_tx: ShutdownRequestsTx,
    ) -> Result<Arc<Self>> {
        let (metrics_exporter, metrics_writer) =
            pomfrit::create_exporter(config.metrics_settings.clone()).await?;

        let context = EngineContext::new(config, global_config, shutdown_requests_tx).await?;

        let engine = Arc::new(Self {
            context,
            _metrics_exporter: metrics_exporter,
        });

        metrics_writer.spawn({
            let engine = Arc::downgrade(&engine);
            move |buffer| {
                let engine = match engine.upgrade() {
                    Some(engine) => engine,
                    None => return,
                };

                buffer
                    .write(LabeledTonServiceMetrics(&engine.context))
                    .write(LabeledTonSubscriberMetrics(&engine.context));
            }
        });

        Ok(engine)
    }

    pub async fn start(self: &Arc<Self>) -> Result<()> {
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
}

pub struct EngineContext {
    pub shutdown_requests_tx: ShutdownRequestsTx,
    pub ton_core: Arc<TonCore>,
    pub ton_client: Arc<TonClientImpl>,
    pub ton_service: Arc<TonServiceImpl>,
    pub auth_service: Arc<AuthServiceImpl>,
    pub memory_storage: Arc<StorageHandler>,
    pub config: AppConfig,
    pub guards: FxDashMap<String, (Arc<Mutex<()>>, u32)>,
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

        let (ton_transaction_tx, ton_transaction_rx) = mpsc::unbounded_channel();
        let (token_transaction_tx, token_transaction_rx) = mpsc::unbounded_channel();

        let ton_core = TonCore::new(
            config.ton_core.clone(),
            global_config,
            sqlx_client.clone(),
            owners_cache,
            ton_transaction_tx,
            token_transaction_tx,
            config.recover_indexer,
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
            guards: Default::default(),
        });

        engine_context.start_listening_ton_transaction(ton_transaction_rx);
        engine_context.start_listening_token_transaction(token_transaction_rx);

        Ok(engine_context)
    }

    async fn start(&self) -> Result<()> {
        self.ton_client.start().await?;
        self.ton_service.start().await?;
        self.ton_core.start().await?;

        Ok(())
    }

    fn start_listening_ton_transaction(self: &Arc<Self>, mut rx: TonTransactionRx) {
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
                        let message_hash = transaction.message_hash.clone();
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
                                log::error!("Failed to create receive transaction with message hash '{}': {:?}", message_hash, err)
                            }
                        }
                    }
                    CaughtTonTransaction::UpdateSent(transaction) => {
                        let guard = engine_context.get_guard(transaction.account_hex.clone());
                        let _lock = guard.lock().await;

                        match engine_context
                            .ton_service
                            .upsert_sent_transaction(
                                transaction.message_hash.clone(),
                                transaction.account_workchain_id,
                                transaction.account_hex.clone(),
                                transaction.input.clone(),
                            )
                            .await
                        {
                            Ok(_) => {
                                match engine_context
                                    .ton_service
                                    .update_token_transaction(
                                        transaction.message_hash.clone(),
                                        transaction.account_workchain_id,
                                        transaction.account_hex,
                                        transaction.input.messages_hash,
                                    )
                                    .await
                                {
                                    Ok(_) => {
                                        state.send(HandleTransactionStatus::Success).ok();
                                    }
                                    Err(err) => {
                                        state.send(HandleTransactionStatus::Fail).ok();
                                        log::error!(
                                            "Failed to update token transaction with message hash '{}': {:?}",
                                            transaction.message_hash,
                                            err
                                        );
                                    }
                                }
                            }
                            Err(err) => {
                                state.send(HandleTransactionStatus::Fail).ok();
                                log::error!(
                                    "Failed to upsert sent transaction with message hash '{}': {:?}",
                                    transaction.message_hash,
                                    err
                                )
                            }
                        }
                    }
                }
            }

            rx.close();
            while rx.recv().await.is_some() {}
        });
    }

    fn start_listening_token_transaction(self: &Arc<Self>, mut rx: TokenTransactionRx) {
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

                let guard = engine_context.get_guard(transaction.account_hex.clone());
                let _lock = guard.lock().await;

                let message_hash = transaction.message_hash.clone();
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
                        log::error!(
                            "Failed to create token transaction with message hash '{}': {:?}",
                            message_hash,
                            e
                        )
                    }
                };
            }

            rx.close();
            while rx.recv().await.is_some() {}
        });
    }

    fn get_guard(&self, account: String) -> Arc<Mutex<()>> {
        use dashmap::mapref::entry::Entry;

        let now = chrono::Utc::now().timestamp() as u32;

        // Delete expired guards
        self.guards.retain(|_, (_, expired_at)| now < *expired_at);

        match self.guards.entry(account) {
            Entry::Occupied(entry) => entry.get().0.clone(),
            Entry::Vacant(entry) => {
                let expired_at = now + 5 * DEFAULT_EXPIRATION_TIMEOUT;
                entry
                    .insert((Arc::new(Mutex::default()), expired_at))
                    .value()
                    .0
                    .clone()
            }
        }
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
