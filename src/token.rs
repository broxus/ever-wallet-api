use anyhow::Result;
use sqlx::postgres::PgPoolOptions;

use crate::models::*;
use crate::settings::*;
use crate::sqlx_client::*;

pub async fn add_root_token(
    config: AppConfig,
    token_name: String,
    token_address: String,
) -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(config.db_pool_size)
        .connect(&config.database_url)
        .await
        .expect("fail pg pool");

    let sqlx_client = SqlxClient::new(pool);
    sqlx_client
        .create_root_token(TokenWhitelistFromDb {
            name: token_name,
            address: token_address,
        })
        .await?;

    log::info!("Root token has been added!");

    Ok(())
}
