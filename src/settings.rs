use std::convert::TryInto;
use std::net::SocketAddr;
use std::path::Path;

use anyhow::{Context, Result};
use argon2::password_hash::PasswordHasher;
use nekoton_utils::TrustMe;
use serde::{Deserialize, Serialize};

use crate::ton_core::*;

#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    /// Listen address of service.
    pub server_addr: SocketAddr,

    /// Postgres database url.
    pub database_url: String,

    /// Postgres connection pools.
    pub db_pool_size: u32,

    /// Key to encrypt/decrypt
    /// accounts private key in db
    #[serde(default = "default_key")]
    pub key: Vec<u8>,

    /// TON node settings
    #[serde(default)]
    pub ton_core: NodeConfig,

    /// Prometheus metrics exporter settings.
    /// Completely disable when not specified
    #[serde(default)]
    pub metrics_settings: Option<pomfrit::Config>,

    /// log4rs settings.
    /// See [docs](https://docs.rs/log4rs/1.0.0/log4rs/) for more details
    #[serde(default = "default_logger_settings")]
    pub logger_settings: serde_yaml::Value,

    /// Recover indexer db
    #[serde(default)]
    pub recover_indexer: bool,
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

fn default_key() -> Vec<u8> {
    fn key() -> Result<Vec<u8>> {
        let secret = std::env::var("SECRET")?;
        let salt = std::env::var("SALT")?;

        let mut options = argon2::ParamsBuilder::default();
        let options = options
            .output_len(32) //chacha key size
            .and_then(|x| x.clone().params())
            .trust_me();

        // Argon2 with default params (Argon2id v19)
        let argon2 =
            argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, options);

        let key = argon2
            .hash_password(secret.as_bytes(), &salt)
            .trust_me()
            .hash
            .context("No hash")?
            .as_bytes()
            .try_into()?;

        Ok(key)
    }

    match key() {
        Ok(key) => key,
        Err(err) => panic!(
            "Failed to get key to encrypt/decrypt private key: {:?}",
            err
        ),
    }
}

fn default_logger_settings() -> serde_yaml::Value {
    const DEFAULT_LOG4RS_SETTINGS: &str = r##"
    appenders:
      stdout:
        kind: console
        encoder:
          pattern: "{d(%Y-%m-%d %H:%M:%S %Z)(utc)} - {h({l})} {M} = {m} {n}"
    root:
      level: info
      appenders:
        - stdout
    loggers:
      ton_wallet_api:
        level: info
        appenders:
          - stdout
        additive: false
    "##;
    serde_yaml::from_str(DEFAULT_LOG4RS_SETTINGS).trust_me()
}
