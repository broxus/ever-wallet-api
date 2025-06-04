use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use everscale_types::dict::Dict;
use everscale_types::models::BlockId;
use futures_util::future::BoxFuture;
use proof_api_l2::api::ApiConfig;
use proof_api_l2::storage::{ProofStorage, ProofStorageConfig};
use proof_api_util::api::Api;
use serde::{Deserialize, Serialize};
use tycho_block_util::archive::ArchiveData;
use tycho_block_util::block::BlockStuff;
use tycho_core::block_strider::{
    ArchiveBlockProvider, BlockProviderExt, BlockSubscriber, BlockSubscriberContext,
    BlockchainBlockProvider, ColdBootType, StateSubscriber, StateSubscriberContext,
    StorageBlockProvider,
};
use tycho_storage::{BlockConnection, BlockHandle, NewBlockMeta, Storage};
use tycho_util::cli::signal;
use tycho_util::futures::JoinTask;

use tycho_wallet_api::api::Api;
use tycho_wallet_api::commands::*;
use tycho_wallet_api::server::*;
use tycho_wallet_api::settings::*;

#[derive(Parser)]
pub struct Cmd {
    #[clap(flatten)]
    pub base: tycho_light_node::CmdRun,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        std::panic::set_hook(Box::new(|info| {
            use std::io::Write;
            let backtrace = std::backtrace::Backtrace::capture();

            tracing::error!("{info}\n{backtrace}");
            std::io::stderr().flush().ok();
            std::io::stdout().flush().ok();
            std::process::exit(1);
        }));

        if let Some(config_path) = self.base.init_config {
            if config_path.exists() && !self.base.force {
                anyhow::bail!("config file already exists, use --force to overwrite");
            }

            let config = NodeConfig {
                rpc: None,
                ..Default::default()
            };

            std::fs::write(config_path, serde_json::to_string_pretty(&config).unwrap())?;
            return Ok(());
        }

        let mut node_config =
            NodeConfig::from_file(self.base.config.as_ref().context("no config")?)
                .context("failed to load node config")?;

        // Always disable RPC by default.
        // TODO: Remove from light nodes.
        node_config.rpc = None;

        tycho_util::cli::logger::init_logger(
            &node_config.logger_config,
            self.base.logger_config.clone(),
        )?;

        rayon::ThreadPoolBuilder::new()
            .stack_size(8 * 1024 * 1024)
            .thread_name(|_| "rayon_worker".to_string())
            .num_threads(node_config.threads.rayon_threads)
            .build_global()
            .unwrap();

        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(node_config.threads.tokio_workers)
            .build()?
            .block_on(async move {
                let run_fut = tokio::spawn(self.run_impl(node_config));
                let stop_fut = signal::any_signal(signal::TERMINATION_SIGNALS);
                tokio::select! {
                    res = run_fut => res.unwrap(),
                    signal = stop_fut => match signal {
                        Ok(signal) => {
                            tracing::info!(?signal, "received termination signal");
                            Ok(())
                        }
                        Err(e) => Err(e.into()),
                    }
                }
            })
    }

    async fn run_impl(self, node_config: NodeConfig) -> Result<()> {
        let import_zerostate = self.base.import_zerostate.clone();

        // Build node.
        let mut node = self.base.create(node_config.clone()).await?;
        tracing::info!("created tycho node");

        let context = EngineContext::new(node_config.user_config).await?;

        context.start().await?;

        let (metrics_exporter, metrics_writer) =
            pomfrit::create_exporter(node_config.user_config.api.node_metrics_settings.clone())
                .await?;

        metrics_writer.spawn({
            let engine = Arc::downgrade(&engine);
            move |buffer| {
                let engine = match engine.upgrade() {
                    Some(engine) => engine,
                    None => return,
                };

                buffer.write(LabeledTonSubscriberMetrics(&engine.context));
            }
        });

        // Bind API.
        let api = Api::bind(
            context.config.server_addr,
            context.config.api_metrics_addr,
            context.auth_service.clone(),
            context.ton_service.clone(),
            context.memory_storage.clone(),
        )
        .await
        .context("failed to bind API service")?;
        tracing::info!("created api");

        // Prepare block providers.
        let archive_block_provider = ArchiveBlockProvider::new(
            node.blockchain_rpc_client().clone(),
            node.storage().clone(),
            node_config.archive_block_provider.clone(),
        );

        let storage_block_provider = StorageBlockProvider::new(node.storage().clone());

        let blockchain_block_provider = BlockchainBlockProvider::new(
            node.blockchain_rpc_client().clone(),
            node.storage().clone(),
            node_config.blockchain_block_provider.clone(),
        )
        .with_fallback(archive_block_provider.clone());

        // Sync node.
        let init_block_id = node
            .init(ColdBootType::LatestPersistent, import_zerostate)
            .await?;

        // Start API
        let api_fut = JoinTask::new(api.serve());

        // Start the node.
        node.run(
            archive_block_provider.chain((blockchain_block_provider, storage_block_provider)),
            LightSubscriber {
                storage: node.storage().clone(),
                context,
            },
        )
        .await?;

        // Serve API for the reset of the lifetime
        api_fut.await.map_err(Into::into)
    }
}

pub struct TempTransaction {
    hash: Vec<u8>,
    transaction: Transaction,
    timestamp: u32,
}

pub struct LightSubscriber {
    storage: Storage,
    context: Arc<EngineContext>,
}

impl LightSubscriber {
    async fn parse_transaction(&self, cx: &TempTransaction) -> Result<()> {}
    async fn prepare_block_impl(&self, cx: &BlockSubscriberContext) -> Result<BlockHandle> {
        let block_stuff = cx.block;
        let block_id = block_stuff.id();

        let extra = block_stuff.load_extra()?;
        let account_blocks = extra.account_blocks.load()?;

        let transactions: anyhow::Result<Vec<Vec<_>>> = tokio::task::spawn_blocking(move || {
            let mut transactions = Vec::new();

            for account_block in account_blocks.iter() {
                let (addr, _, block) = account_block?;
                for transaction in block.transactions.iter() {
                    let (_, _, transaction) = transaction?;
                    let hash = transaction.inner().repr_hash().0.to_vec();
                    let transaction = transaction.load()?;
                    let timestamp = transaction.now;

                    let partition = if block_id.is_masterchain() {
                        0
                    } else {
                        // first 3 bits of the account id
                        1 + (addr[0] >> 5)
                    };

                    transactions.push(TempTransaction {
                        hash,
                        transaction,
                        timestamp,
                    });
                }
            }
            Ok(transactions)
        })
        .await?;
        let transactions = transactions?;

        let mut futures: FuturesOrdered<_> = transactions
            .into_iter()
            .flatten()
            .map(move |tx| self.parse_transaction(tx))
            .collect();

        while futures.next().await.is_some() {}

        Ok(handle)
    }

    async fn handle_block_impl(
        &self,
        cx: &BlockSubscriberContext,
        handle: BlockHandle,
    ) -> Result<()> {
        tracing::info!(
            block_id = %cx.block.id(),
            mc_block_id = %cx.mc_block_id,
            "handling block"
        );

        // Done
        Ok(())
    }
}

impl BlockSubscriber for LightSubscriber {
    type Prepared = BlockHandle;

    type PrepareBlockFut<'a> = BoxFuture<'a, Result<Self::Prepared>>;
    type HandleBlockFut<'a> = BoxFuture<'a, Result<()>>;

    fn prepare_block<'a>(&'a self, cx: &'a BlockSubscriberContext) -> Self::PrepareBlockFut<'a> {
        Box::pin(self.prepare_block_impl(cx))
    }

    fn handle_block<'a>(
        &'a self,
        cx: &'a BlockSubscriberContext,
        handle: Self::Prepared,
    ) -> Self::HandleBlockFut<'a> {
        Box::pin(self.handle_block_impl(cx, handle))
    }
}

impl StateSubscriber for LightSubscriber {
    type HandleStateFut<'a> = futures_util::future::Ready<Result<()>>;

    fn handle_state<'a>(&'a self, cx: &'a StateSubscriberContext) -> Self::HandleStateFut<'a> {
        futures_util::future::ready(self.inner.update_accounts_cache(&cx.block, &cx.state))
    }
}

type NodeConfig = tycho_light_node::NodeConfig<NodeConfigExtra>;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct NodeConfigExtra {
    pub api: AppConfig,
}
