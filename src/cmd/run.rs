use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tycho_core::block_strider::ShardStateApplier;

use tycho_core::block_strider::MetricsSubscriber;
use tycho_core::block_strider::{BlockProviderExt, ColdBootType};
use tycho_core::blockchain_rpc::NoopBroadcastListener;
use tycho_core::global_config::GlobalConfig;
use tycho_core::node::{NodeBase, NodeBaseConfig, NodeBootArgs, NodeKeys};
use tycho_util::cli;
use tycho_util::cli::config::ThreadPoolConfig;
use tycho_util::cli::logger::LoggerConfig;
use tycho_util::cli::metrics::MetricsConfig;
use tycho_util::config::PartialConfig;
use tycho_util::futures::JoinTask;
use tycho_util::serde_helpers::{load_json_from_file, save_json_to_file};

use tycho_wallet_api::api::Api;
use tycho_wallet_api::server::*;
use tycho_wallet_api::settings::*;

#[derive(Parser)]
pub struct Cmd {
    /// dump the node config template
    #[clap(
        short = 'i',
        long,
        conflicts_with_all = ["config", "global_config", "keys", "logger_config", "import_zerostate", "cold_boot"]
    )]
    pub init_config: Option<PathBuf>,

    #[clap(
        long,
        short,
        conflicts_with_all = ["config", "global_config", "keys", "logger_config", "import_zerostate", "cold_boot"]
    )]
    pub all: bool,

    /// overwrite the existing config
    #[clap(short, long)]
    pub force: bool,

    /// path to the node config
    #[clap(long, required_unless_present = "init_config")]
    pub config: Option<PathBuf>,

    /// path to the global config
    #[clap(long, required_unless_present = "init_config")]
    pub global_config: Option<PathBuf>,

    /// path to node keys
    #[clap(long, required_unless_present = "init_config")]
    pub keys: Option<PathBuf>,

    /// path to the logger config
    #[clap(long)]
    pub logger_config: Option<PathBuf>,

    /// list of zerostate files to import
    #[clap(long)]
    pub import_zerostate: Option<Vec<PathBuf>>,

    /// Overwrite cold boot type. Default: `latest-persistent`
    #[clap(long)]
    pub cold_boot: Option<ColdBootType>,
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

        if let Some(config_path) = self.init_config {
            if config_path.exists() && !self.force {
                anyhow::bail!("config file already exists, use --force to overwrite");
            }

            let config = NodeConfig::default();
            return if self.all {
                save_json_to_file(config, config_path)
            } else {
                save_json_to_file(config.into_partial(), config_path)
            };
        }

        let node_config: NodeConfig =
            load_json_from_file(self.config.as_ref().context("no config")?)
                .context("failed to load node config")?;

        cli::logger::init_logger(&node_config.logger_config, self.logger_config.clone())?;
        cli::logger::set_abort_with_tracing();

        node_config.threads.init_reclaimer().unwrap();
        node_config.threads.init_global_rayon_pool().unwrap();
        node_config
            .threads
            .build_tokio_runtime()?
            .block_on(cli::signal::run_or_terminate(self.run_impl(node_config)))
    }

    async fn run_impl(self, node_config: NodeConfig) -> Result<()> {
        if let Some(metrics) = &node_config.metrics {
            tycho_util::cli::metrics::init_metrics(metrics)?;
        }

        // Build node.
        let keys = NodeKeys::load_or_create(self.keys.unwrap())?;
        let global_config = GlobalConfig::from_file(self.global_config.unwrap())
            .context("failed to load global config")?;
        let public_ip = cli::resolve_public_ip(node_config.base.public_ip).await?;
        let public_addr = SocketAddr::new(public_ip, node_config.base.port);

        let node = NodeBase::builder(&node_config.base, &global_config)
            .init_network(public_addr, &keys.as_secret())?
            .init_storage()
            .await?
            .init_blockchain_rpc(NoopBroadcastListener, NoopBroadcastListener)?
            .build()?;

        let context = EngineContext::new(
            node_config.api,
            node.core_storage.clone(),
            node.blockchain_rpc_client.clone(),
        )
        .await?;

        // Sync node.
        node.wait_for_neighbours(3).await;

        let boot_type = self.cold_boot.unwrap_or(ColdBootType::LatestPersistent);
        let init_block_id = node
            .boot_ext(NodeBootArgs {
                boot_type,
                zerostates: self.import_zerostate,
                queue_state_handler: None,
                ignore_states: false,
            })
            .await?;
        tracing::info!(%init_block_id, "node initialized");
        node.update_validator_set_from_shard_state(&init_block_id)
            .await?;

        // Build strider.
        let archive_block_provider = node.build_archive_block_provider();
        let storage_block_provider = node.build_storage_block_provider();
        let blockchain_block_provider = node
            .build_blockchain_block_provider()
            .with_fallback(archive_block_provider.clone());

        let block_strider = node.build_strider(
            archive_block_provider.chain((blockchain_block_provider, storage_block_provider)),
            (
                ShardStateApplier::new(node.core_storage.clone(), context.clone()),
                MetricsSubscriber,
            ),
        );

        context
            .start(&init_block_id)
            .await
            .context("failed to start context")?;

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
        // Start API
        let api_fut = JoinTask::new(api.serve());

        // Run block strider
        tracing::info!("block strider started");
        tokio::select! {
            res = block_strider.run() => res?,
            res = api_fut => res?
        }
        tracing::info!("block strider finished");

        Ok(())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialConfig)]
#[serde(default)]
struct NodeConfig {
    #[partial]
    #[serde(flatten)]
    base: NodeBaseConfig,
    #[important]
    threads: ThreadPoolConfig,
    #[important]
    logger_config: LoggerConfig,
    #[important]
    metrics: Option<MetricsConfig>,
    #[important]
    api: AppConfig,
}
