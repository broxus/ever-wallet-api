use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tycho_core::block_strider::ShardStateApplier;
use tycho_core::block_strider::{
    ArchiveBlockProvider, BlockProviderExt, BlockchainBlockProvider, ColdBootType,
    StorageBlockProvider,
};
use tycho_util::cli::signal;
use tycho_util::futures::JoinTask;

use tycho_wallet_api::api::Api;
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

        let context = EngineContext::new(
            node_config.user_config.api,
            node.storage().clone(),
            node.blockchain_rpc_client().clone(),
        )
        .await?;

        // Bind API.
        let api = Api::bind(
            context.config.server_addr,
            context.config.public_url.clone(),
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
        let _ = node
            .init(ColdBootType::LatestPersistent, import_zerostate)
            .await?;

        // Start API
        let api_fut = JoinTask::new(api.serve());

        // Start the node.
        node.run(
            archive_block_provider.chain((blockchain_block_provider, storage_block_provider)),
            ShardStateApplier::new(node.storage().clone(), context.clone()),
        )
        .await?;

        context.start().await?;

        // Serve API for the reset of the lifetime
        api_fut.await.map_err(Into::into)
    }
}

type NodeConfig = tycho_light_node::NodeConfig<NodeConfigExtra>;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct NodeConfigExtra {
    pub api: AppConfig,
}
