use crate::models::*;
use crate::sqlx_client::*;

impl SqlxClient {
    pub async fn get_root_token(
        &self,
        address: &str,
    ) -> Result<TokenWhitelistFromDb, anyhow::Error> {
        let res = sqlx::query_as!(
            TokenWhitelistFromDb,
            r#"SELECT name, address, version as "version: _"
                FROM token_whitelist
                WHERE address = $1"#,
            address
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(res)
    }

    pub async fn create_root_token(
        &self,
        root_token: TokenWhitelistFromDb,
    ) -> Result<TokenWhitelistFromDb, anyhow::Error> {
        sqlx::query_as!(
            TokenWhitelistFromDb,
            r#"INSERT INTO token_whitelist
                (name, address, version)
                VALUES ($1, $2, $3::twa_token_wallet_version)
                RETURNING
                name, address, version as "version: _" "#,
            root_token.name,
            root_token.address,
            root_token.version as TokenWalletVersionDb,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }
}
