use std::str::FromStr;

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
    let root_token = sqlx_client
        .create_root_token(TokenWhitelistFromDb {
            name: token_name,
            address: token_address,
        })
        .await?;

    log::info!("Root token {:?} has been added!", root_token);

    Ok(())
}

pub async fn create_api_service(
    config: AppConfig,
    service_name: String,
    service_id: Option<String>,
) -> Result<()> {
    let id = match service_id {
        Some(id) => ServiceId::from_str(&id)?,
        None => ServiceId::generate(),
    };

    let pool = PgPoolOptions::new()
        .max_connections(config.db_pool_size)
        .connect(&config.database_url)
        .await
        .expect("fail pg pool");

    let sqlx_client = SqlxClient::new(pool);
    let api_service = sqlx_client.create_api_service(id, &service_name).await?;

    log::info!("Api service {:?} created successfully!", api_service);

    Ok(())
}

pub async fn create_api_service_key(
    config: AppConfig,
    service_id: String,
    service_key: String,
    service_secret: String,
) -> Result<()> {
    let service_id = ServiceId::from_str(&service_id)?;

    let pool = PgPoolOptions::new()
        .max_connections(config.db_pool_size)
        .connect(&config.database_url)
        .await
        .expect("fail pg pool");

    let sqlx_client = SqlxClient::new(pool);
    let api_service_key = sqlx_client
        .create_api_service_key(service_id, &service_key, &service_secret)
        .await?;

    log::info!(
        "Api service key {:?} created successfully!",
        api_service_key
    );

    Ok(())
}
