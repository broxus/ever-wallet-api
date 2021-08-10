use std::net::SocketAddr;

use config::{Config as RawConfig, ConfigError, Environment};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server_addr: SocketAddr,
    pub healthcheck_addr: SocketAddr,
    pub database_url: String,
    pub db_pool_size: u32,
    pub redis_addr: String,
}

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = RawConfig::new();
        s.merge(Environment::new())?;

        s.try_into()
    }
}
