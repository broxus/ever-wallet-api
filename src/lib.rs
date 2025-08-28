#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::inconsistent_struct_constructor)]

use std::sync::OnceLock;

pub mod api;
pub mod client;
pub mod commands;
pub mod models;
pub mod prelude;
pub mod server;
pub mod services;
pub mod settings;
pub mod sqlx_client;
pub mod ton_core;
pub mod utils;

pub static BIN_VERSION: &str = "TYCHO_WALLET_API_VERSION";
pub static BIN_BUILD: &str = "TYCHO_WALLET_API_BUILD";

pub fn version_string() -> &'static str {
    static STRING: OnceLock<String> = OnceLock::new();
    STRING.get_or_init(|| format!("(release {BIN_VERSION}) (build {BIN_BUILD})"))
}
