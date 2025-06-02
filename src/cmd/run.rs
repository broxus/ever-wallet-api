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
    BlockchainBlockProvider, ColdBootType, StorageBlockProvider,
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
            pomfrit::create_exporter(node_config.user_config.api.node_metrics_settings.clone()).await?;

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

        let engine = Engine::new(config, global_config, shutdown_requests_tx)
            .await
            .context("Failed to create engine")?;

        engine.start().await.context("Failed to start engine")?;

        // Start the node.
        node.run(
            archive_block_provider.chain((blockchain_block_provider, storage_block_provider)),
            LightSubscriber {
                storage: node.storage().clone(),
                proofs,
            },
        )
        .await?;

        // Serve API for the reset of the lifetime
        api_fut.await.map_err(Into::into)
    }
}

pub struct LightSubscriber {
    storage: Storage,
    proofs: ProofStorage,
}

impl LightSubscriber {
    async fn get_block_handle(
        &self,
        mc_block_id: &BlockId,
        block: &BlockStuff,
        archive_data: &ArchiveData,
    ) -> Result<BlockHandle> {
        let block_storage = self.storage.block_storage();

        let info = block.load_info()?;
        let res = block_storage
            .store_block_data(block, archive_data, NewBlockMeta {
                is_key_block: info.key_block,
                gen_utime: info.gen_utime,
                ref_by_mc_seqno: mc_block_id.seqno,
            })
            .await?;

        Ok(res.handle)
    }

    async fn prepare_block_impl(&self, cx: &BlockSubscriberContext) -> Result<BlockHandle> {
        tracing::info!(
            mc_block_id = %cx.mc_block_id.as_short_id(),
            id = %cx.block.id(),
            "preparing block",
        );

        // Load handle
        let handle = self
            .get_block_handle(&cx.mc_block_id, &cx.block, &cx.archive_data)
            .await?;

        let (prev_id, prev_id_alt) = cx
            .block
            .construct_prev_id()
            .context("failed to construct prev id")?;

        // Update block connections
        let block_handles = self.storage.block_handle_storage();
        let connections = self.storage.block_connection_storage();

        let block_id = cx.block.id();

        let prev_handle = block_handles.load_handle(&prev_id);

        match prev_id_alt {
            None => {
                if let Some(handle) = prev_handle {
                    let direction = if block_id.shard != prev_id.shard
                        && prev_id.shard.split().unwrap().1 == block_id.shard
                    {
                        // Special case for the right child after split
                        BlockConnection::Next2
                    } else {
                        BlockConnection::Next1
                    };
                    connections.store_connection(&handle, direction, block_id);
                }
                connections.store_connection(&handle, BlockConnection::Prev1, &prev_id);
            }
            Some(ref prev_id_alt) => {
                if let Some(handle) = prev_handle {
                    connections.store_connection(&handle, BlockConnection::Next1, block_id);
                }
                if let Some(handle) = block_handles.load_handle(prev_id_alt) {
                    connections.store_connection(&handle, BlockConnection::Next1, block_id);
                }
                connections.store_connection(&handle, BlockConnection::Prev1, &prev_id);
                connections.store_connection(&handle, BlockConnection::Prev2, prev_id_alt);
            }
        }

        // Get block signatures for masterchain block.
        let signatures = if cx.block.id().is_masterchain() {
            let proof = self
                .storage
                .block_storage()
                .load_block_proof(&handle)
                .await?;
            let Some(signatures) = &proof.as_ref().signatures else {
                anyhow::bail!("masterchain block proof without signatures: {block_id}");
            };
            signatures.signatures.clone()
        } else {
            Dict::new()
        };

        // Store proof.
        self.proofs
            .store_block(cx.block.clone(), signatures, cx.mc_block_id.seqno)
            .await?;

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

        // Save block to archive.
        if self.storage.config().archives_gc.is_some() {
            tracing::debug!(block_id = %handle.id(), "saving block into archive");
            self.storage
                .block_storage()
                .move_into_archive(&handle, cx.mc_is_key_block)
                .await?;
        }

        // Update proofs storage snapshot on masterchain blocks.
        if cx.block.id().is_masterchain() {
            self.proofs.update_snapshot();
        }

        // Update current vset on key blocks.
        if cx.is_key_block {
            let custom = cx.block.load_custom()?;
            let config = custom.config.as_ref().context("key block without config")?;

            let current_vset = config
                .get_current_validator_set()
                .context("failed to get current validator set")
                .map(Arc::new)?;

            self.proofs.set_current_vset(current_vset);
        }

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

type NodeConfig = tycho_light_node::NodeConfig<NodeConfigExtra>;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct NodeConfigExtra {
    pub api: AppConfig,
}
