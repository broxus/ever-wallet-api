use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::PathBuf;

use anyhow::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// TON node settings
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NodeConfig {
    /// Node public ip. Automatically determines if None
    pub adnl_public_ip: Option<Ipv4Addr>,

    /// Node port. Default: 30303
    pub adnl_port: u16,

    /// Path to the DB directory. Default: `./db`
    pub db_path: PathBuf,

    /// Path to the ADNL keys. Default: `./adnl-keys.json`.
    /// NOTE: generates new keys if specified path doesn't exist
    pub keys_path: PathBuf,

    /// Allowed DB size in bytes. Default: one third of all machine RAM
    pub max_db_memory_usage: usize,

    /// Archives map queue. Default: 16
    pub parallel_archive_downloads: u32,

    /// Whether old shard states will be removed every 10 minutes
    pub states_gc_enabled: bool,

    /// Whether old blocks will be removed on each new key block
    pub blocks_gc_enabled: bool,
}

impl NodeConfig {
    pub async fn build_indexer_config(self) -> Result<ton_indexer::NodeConfig> {
        // Determine public ip
        let ip_address = match self.adnl_public_ip {
            Some(address) => address,
            None => public_ip::addr_v4()
                .await
                .ok_or(ConfigError::PublicIpNotFound)?,
        };

        log::info!("Using public ip: {}", ip_address);

        let adnl_keys = match read_adnl_keys(self.keys_path.clone()) {
            Ok(keys) => keys,
            Err(err) => {
                log::warn!("Generate a new NodeKeys config for a reason: {}", err);
                generate_adnl_keys(self.keys_path.clone()).await?;
                read_adnl_keys(self.keys_path.clone())?
            }
        };

        // Prepare DB folder
        std::fs::create_dir_all(&self.db_path)?;

        // Done
        Ok(ton_indexer::NodeConfig {
            ip_address: SocketAddrV4::new(ip_address, self.adnl_port),
            adnl_keys,
            rocks_db_path: self.db_path.join("rocksdb"),
            file_db_path: self.db_path.join("files"),
            state_gc_options: self.states_gc_enabled.then(|| ton_indexer::StateGcOptions {
                offset_sec: rand::thread_rng().gen_range(0..3600),
                interval_sec: 3600,
            }),
            blocks_gc_options: self
                .blocks_gc_enabled
                .then(|| ton_indexer::BlocksGcOptions {
                    kind: ton_indexer::BlocksGcKind::BeforePreviousKeyBlock,
                    enable_for_sync: true,
                    ..Default::default()
                }),
            shard_state_cache_options: None,
            archives_enabled: false,
            old_blocks_policy: Default::default(),
            max_db_memory_usage: self.max_db_memory_usage,
            parallel_archive_downloads: self.parallel_archive_downloads,
            adnl_options: Default::default(),
            rldp_options: Default::default(),
            dht_options: Default::default(),
            neighbours_options: Default::default(),
            overlay_shard_options: tiny_adnl::OverlayShardOptions {
                broadcast_target_count: 10,
                ..Default::default()
            },
        })
    }
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            adnl_public_ip: None,
            adnl_port: 30303,
            db_path: "db".into(),
            keys_path: "adnl-keys.json".into(),
            max_db_memory_usage: ton_indexer::default_max_db_memory_usage(),
            parallel_archive_downloads: 16,
            states_gc_enabled: true,
            blocks_gc_enabled: true,
        }
    }
}

async fn generate_adnl_keys<T>(path: T) -> Result<()>
where
    T: AsRef<std::path::Path>,
{
    use std::io::Write;

    let mut file = std::fs::File::create(path)?;
    let config = ton_indexer::NodeKeys::generate();
    file.write_all(serde_yaml::to_string(&config)?.as_bytes())?;
    Ok(())
}

fn read_adnl_keys(path: PathBuf) -> Result<ton_indexer::NodeKeys> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let config = serde_yaml::from_reader(reader)?;
    Ok(config)
}

#[derive(thiserror::Error, Debug)]
enum ConfigError {
    #[error("Failed to find public ip")]
    PublicIpNotFound,
}
