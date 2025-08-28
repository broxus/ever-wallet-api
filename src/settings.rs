use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;

use anyhow::{Context, Result};
use argon2::password_hash::PasswordHasher;
use nekoton_utils::TrustMe;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    /// Listen address of service.
    #[serde(default = "default_server_addr")]
    pub server_addr: SocketAddr,

    /// Postgres database url.
    pub database_url: String,

    /// Postgres connection pools.
    pub db_pool_size: u32,

    /// Key to encrypt/decrypt
    /// accounts private key in db
    #[serde(default = "default_key")]
    pub key: Vec<u8>,

    /// API prometheus metrics exporter settings.
    /// Completely disable when not specified
    #[serde(default)]
    pub api_metrics_addr: Option<SocketAddr>,

    /// Node prometheus metrics exporter settings.
    /// Completely disable when not specified
    #[serde(default)]
    pub node_metrics_settings: Option<pomfrit::Config>,

    /// Public url of service
    pub public_url: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_addr: default_server_addr(),
            database_url: "postgresql://postgres:postgres@127.0.0.1:5432/tycho_wallet_api"
                .to_string(),
            db_pool_size: 8,
            key: default_key(),
            api_metrics_addr: Default::default(),
            node_metrics_settings: Default::default(),
            public_url: Default::default(),
        }
    }
}

pub trait ConfigExt: Sized {
    fn from_file<P>(path: &P) -> Result<Self>
    where
        P: AsRef<Path>;
}

fn default_server_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)
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
            .into();

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
