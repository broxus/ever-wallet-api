use std::net::{IpAddr, SocketAddrV4};
use std::path::PathBuf;

use anyhow::Result;
use ed25519_dalek::SecretKey;
use serde::{Deserialize, Serialize};

use super::TonCoreConfig;

pub async fn get_node_config(config: &TonCoreConfig) -> Result<ton_indexer::NodeConfig> {
    let ip = external_ip::ConsensusBuilder::new()
        .add_sources(external_ip::get_http_sources::<external_ip::Sources>())
        .build()
        .get_consensus()
        .await;

    let ip_address = match ip {
        Some(IpAddr::V4(ip)) => SocketAddrV4::new(ip, config.port),
        Some(_) => anyhow::bail!("IPv6 not supported"),
        None => anyhow::bail!("External ip not found"),
    };

    let adnl_keys = match read_keys_config(config.keys_path.clone()) {
        Ok(keys) => keys,
        Err(err) => {
            log::warn!("Generate a new NodeKeys config for a reason: {}", err);
            generate_keys_config(config.keys_path.clone()).await?;
            read_keys_config(config.keys_path.clone())?
        }
    };

    Ok(ton_indexer::NodeConfig {
        ip_address,
        adnl_keys,
        rocks_db_path: config.rocks_db_path.clone(),
        file_db_path: config.file_db_path.clone(),
        old_blocks_policy: Default::default(),
        shard_state_cache_enabled: false,
        max_db_memory_usage: ton_indexer::default_max_db_memory_usage(),
        adnl_options: Default::default(),
        rldp_options: Default::default(),
        dht_options: Default::default(),
        neighbours_options: Default::default(),
        overlay_shard_options: Default::default(),
    })
}

async fn generate_keys_config<T>(path: T) -> Result<()>
where
    T: AsRef<std::path::Path>,
{
    use std::io::Write;

    let mut file = std::fs::File::create(path)?;
    let config = ton_indexer::NodeKeys::generate();
    file.write_all(serde_yaml::to_string(&config)?.as_bytes())?;
    Ok(())
}

fn read_keys_config(path: PathBuf) -> Result<ton_indexer::NodeKeys> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let config = serde_yaml::from_reader(reader)?;
    Ok(config)
}
