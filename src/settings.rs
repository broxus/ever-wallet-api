use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::ton_core::*;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub server_addr: String,
    pub database_url: String,
    pub db_pool_size: u32,
    pub secret: String,
    pub ton_core: NodeConfig,
    #[serde(default = "default_logger_settings")]
    pub logger_settings: serde_yaml::Value,
}

impl ConfigExt for Config {
    fn from_file<P>(path: &P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut config = config::Config::new();
        config.merge(read_config(path).context("Failed to read config")?)?;
        config.merge(config::Environment::new())?;
        Ok(config.try_into()?)
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
    serde_yaml::from_str(DEFAULT_LOG4RS_SETTINGS).unwrap()
}

fn read_config<P>(path: P) -> Result<config::File<config::FileSourceString>>
where
    P: AsRef<std::path::Path>,
{
    let data = std::fs::read_to_string(path)?;
    let re = regex::Regex::new(r"\$\{([a-zA-Z_][0-9a-zA-Z_]*)\}").unwrap();
    let result = re.replace_all(&data, |caps: &regex::Captures| {
        match std::env::var(&caps[1]) {
            Ok(value) => value,
            Err(_) => (&caps[0]).to_string(),
        }
    });

    Ok(config::File::from_str(
        result.as_ref(),
        config::FileFormat::Yaml,
    ))
}
