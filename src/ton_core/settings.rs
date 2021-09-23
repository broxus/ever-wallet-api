use std::net::{IpAddr, SocketAddrV4};
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// TON node settings
#[derive(Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NodeConfig {
    /// Node port. Default: 30303
    pub adnl_port: u16,

    /// Path to the DB directory. Default: `./db`
    pub db_path: PathBuf,

    /// Path to the ADNL keys. Default: `./adnl-keys.json`.
    /// NOTE: generates new keys if specified path doesn't exist
    pub keys_path: PathBuf,

    /// Allowed DB size in bytes. Default: one third of all machine RAM
    pub max_db_memory_usage: usize,
}

impl NodeConfig {
    pub async fn build_indexer_config(self) -> Result<ton_indexer::NodeConfig> {
        let ip = external_ip::ConsensusBuilder::new()
            .add_sources(external_ip::get_http_sources::<external_ip::Sources>())
            .build()
            .get_consensus()
            .await;

        let ip_address = match ip {
            Some(IpAddr::V4(ip)) => SocketAddrV4::new(ip, self.adnl_port),
            Some(_) => anyhow::bail!("IPv6 not supported"),
            None => anyhow::bail!("External ip not found"),
        };
        log::info!("Using public ip: {}", ip_address);

        // temp decision to recreate adnl keys
        std::fs::remove_file(self.keys_path.clone())?;

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
            ip_address,
            adnl_keys,
            rocks_db_path: self.db_path.join("rocksdb"),
            file_db_path: self.db_path.join("file"),
            old_blocks_policy: Default::default(),
            shard_state_cache_enabled: false,
            max_db_memory_usage: self.max_db_memory_usage,
            adnl_options: Default::default(),
            rldp_options: Default::default(),
            dht_options: Default::default(),
            neighbours_options: Default::default(),
            overlay_shard_options: Default::default(),
        })
    }
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            adnl_port: 30303,
            db_path: "db".into(),
            keys_path: "adnl-keys.json".into(),
            max_db_memory_usage: ton_indexer::default_max_db_memory_usage(),
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
