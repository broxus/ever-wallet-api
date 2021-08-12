use std::fs::File;
use std::net::SocketAddr;
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

use crate::indexer::IndexerConfig;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub server_addr: SocketAddr,
    pub healthcheck_addr: SocketAddr,
    pub database_url: String,
    pub db_pool_size: u32,
    pub redis_addr: String,
    pub indexer: IndexerConfig,
}

impl ConfigExt for Config {
    fn from_file<P>(path: &P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let config = serde_yaml::from_reader(reader)?;
        Ok(config)
    }
}

impl ConfigExt for ton_indexer::GlobalConfig {
    fn from_file<P>(path: &P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let config = serde_json::from_reader(reader)?;
        Ok(config)
    }
}

pub trait ConfigExt: Sized {
    fn from_file<P>(path: &P) -> Result<Self>
    where
        P: AsRef<Path>;
}
