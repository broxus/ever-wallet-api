use std::borrow::Cow;
use std::env;
use std::fs::File;
use std::path::Path;

use anyhow::Result;
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

impl Config {
    pub fn load_env(mut self) -> Self {
        self.database_url = expand_env(&self.database_url).into_owned();
        self.secret = expand_env(&self.secret).into_owned();
        self
    }
}

impl ConfigExt for Config {
    fn from_file<P>(path: &P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let config: Config = serde_yaml::from_reader(reader)?;
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

pub fn expand_env(raw_config: &str) -> Cow<str> {
    let re = regex::Regex::new(r"\$\{([a-zA-Z_][0-9a-zA-Z_]*)\}").unwrap();
    re.replace_all(raw_config, |caps: &regex::Captures| {
        match env::var(&caps[1]) {
            Ok(val) => val,
            Err(_) => (&caps[0]).to_string(),
        }
    })
}
