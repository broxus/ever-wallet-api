use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use futures::future::BoxFuture;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tycho_core::block_strider::StateSubscriber;
use tycho_core::block_strider::StateSubscriberContext;
use tycho_core::blockchain_rpc::BlockchainRpcClient;
use tycho_core::storage::CoreStorage;
use tycho_types::models::BlockId;

use crate::client::*;
use crate::models::*;
use crate::prelude::*;
use crate::services::*;
use crate::settings::*;
use crate::sqlx_client::*;
use crate::ton_core::*;
use crate::utils::*;

pub struct EngineContext {
    pub auth_service: Arc<AuthService>,
    pub ton_core: Arc<TonCore>,
    pub ton_client: Arc<TonClient>,
    pub ton_service: Arc<TonService>,
    pub memory_storage: Arc<StorageHandler>,
    pub config: AppConfig,
    pub guards: FxDashMap<String, (Arc<Mutex<()>>, u32)>,
}

impl EngineContext {
    pub async fn new(
        config: AppConfig,
        storage: CoreStorage,
        blockchain_rpc_client: BlockchainRpcClient,
    ) -> Result<Arc<Self>> {
        let pool = PgPoolOptions::new()
            .max_connections(config.db_pool_size)
            .connect(&config.database_url)
            .await
            .expect("fail pg pool");

        sqlx::migrate!().run(&pool).await?;

        let sqlx_client = SqlxClient::new(pool);

        let callback_client = Arc::new(CallbackClient::new());
        let owners_cache = OwnersCache::new(sqlx_client.clone()).await?;

        let (ton_transaction_tx, ton_transaction_rx) = mpsc::unbounded_channel();
        let (token_transaction_tx, token_transaction_rx) = mpsc::unbounded_channel();

        let ton_core = TonCore::new(
            sqlx_client.clone(),
            owners_cache,
            ton_transaction_tx,
            token_transaction_tx,
            storage,
            blockchain_rpc_client,
        )
        .await?;

        let ton_client = Arc::new(TonClient::new(ton_core.clone(), sqlx_client.clone()));

        let ton_service = Arc::new(TonService::new(
            sqlx_client.clone(),
            ton_client.clone(),
            callback_client.clone(),
            config.key.clone(),
        ));

        let auth_service = Arc::new(AuthService::new(sqlx_client.clone()));

        let memory_storage = Arc::new(StorageHandler::default());

        let engine_context = Arc::new(Self {
            auth_service,
            ton_core,
            ton_client,
            ton_service,
            memory_storage,
            config,
            guards: Default::default(),
        });

        engine_context.start_listening_ton_transaction(ton_transaction_rx);
        engine_context.start_listening_token_transaction(token_transaction_rx);
        engine_context.start_updating_accounts_subscription();

        Ok(engine_context)
    }

    pub async fn start(&self, last_block_id: &BlockId) -> Result<()> {
        self.ton_client
            .start()
            .await
            .context("failed to start ton_client")?;
        self.ton_service
            .start()
            .await
            .context("failed to start ton_service")?;
        self.ton_core
            .start(last_block_id)
            .await
            .context("failed to start ton_core")?;

        Ok(())
    }

    fn start_listening_ton_transaction(self: &Arc<Self>, mut rx: TonTransactionRx) {
        let engine_context = Arc::downgrade(self);

        tokio::spawn(async move {
            while let Some((transaction, state)) = rx.recv().await {
                let engine_context = match engine_context.upgrade() {
                    Some(engine_context) => engine_context,
                    None => {
                        tracing::error!("Engine is already dropped");
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
                                tracing::error!("Failed to create receive transaction with message hash '{}': {:?}", message_hash, err)
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
                                        tracing::error!(
                                            "Failed to update token transaction with message hash '{}': {:?}",
                                            transaction.message_hash,
                                            err
                                        );
                                    }
                                }
                            }
                            Err(err) => {
                                state.send(HandleTransactionStatus::Fail).ok();
                                tracing::error!(
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
                        tracing::error!("Engine is already dropped");
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
                        tracing::error!(
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

    fn start_updating_accounts_subscription(self: &Arc<Self>) {
        let engine_context = Arc::downgrade(self);

        tokio::spawn(async move {
            loop {
                let engine_context = match engine_context.upgrade() {
                    Some(engine_context) => engine_context,
                    None => {
                        tracing::error!("Engine is already dropped");
                        return;
                    }
                };

                engine_context
                    .ton_service
                    .resubscribe_for_accounts_created_last_minute()
                    .await
                    .unwrap_or_else(|e| {
                        tracing::error!("Failed to update accounts: {}", e);
                    });
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            }
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

pub type ShutdownRequestsRx = mpsc::UnboundedReceiver<()>;
pub type ShutdownRequestsTx = mpsc::UnboundedSender<()>;

impl StateSubscriber for EngineContext {
    type HandleStateFut<'a> = BoxFuture<'a, Result<()>>;

    fn handle_state<'a>(&'a self, cx: &'a StateSubscriberContext) -> Self::HandleStateFut<'a> {
        Box::pin(
            self.ton_core
                .context
                .ton_subscriber
                .process_block(&cx.block, &cx.state),
        )
    }
}
