use std::net::{IpAddr, SocketAddrV4};
use std::path::PathBuf;

use anyhow::Result;
use ed25519_dalek::SecretKey;
use serde::{Deserialize, Serialize};

use super::TonCoreConfig;

const MAX_DB_MEMTABLES_SIZE: usize = 256 * 1024 * 1024;

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

    if !config.keys_path.exists() {
        generate_keys_config(config.keys_path.clone()).await?
    }

    let keys_config = read_keys_config(config.keys_path.clone())?;

    Ok(ton_indexer::NodeConfig {
        ip_address,
        keys: keys_config.entries,
        rocks_db_path: config.rocks_db_path.clone(),
        file_db_path: config.file_db_path.clone(),
        shard_state_cache_enabled: false,
        initial_sync_before: 300,
        max_db_memtables_size: MAX_DB_MEMTABLES_SIZE,
    })
}

#[derive(Deserialize, Serialize)]
struct KeysConfig {
    entries: Vec<ton_indexer::AdnlNodeKey>,
}

impl KeysConfig {
    pub fn generate() -> Self {
        let mut keys = Vec::new();

        let get_node_key = |tag: usize| -> ton_indexer::AdnlNodeKey {
            ton_indexer::AdnlNodeKey {
                tag,
                key: SecretKey::generate(&mut rand::thread_rng()).to_bytes(),
            }
        };

        keys.push(get_node_key(1));
        keys.push(get_node_key(2));

        KeysConfig { entries: keys }
    }
}

async fn generate_keys_config<T>(path: T) -> Result<()>
where
    T: AsRef<std::path::Path>,
{
    use std::io::Write;

    let mut file = std::fs::File::create(path)?;
    let config = KeysConfig::generate();
    file.write_all(serde_yaml::to_string(&config)?.as_bytes())?;
    Ok(())
}

fn read_keys_config(path: PathBuf) -> Result<KeysConfig> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let config = serde_yaml::from_reader(reader)?;
    Ok(config)
}
