use crate::models::*;
use crate::sqlx_client::*;

impl SqlxClient {
    pub async fn get_token_whitelist(&self) -> Result<Vec<TokenWhitelistFromDb>, anyhow::Error> {
        let res = sqlx::query_as!(
            TokenWhitelistFromDb,
            r#"SELECT name, address FROM token_whitelist"#
        )
        .fetch_all(&self.pool)
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
                (name, address)
                VALUES ($1, $2)
                RETURNING
                name, address"#,
            root_token.name,
            root_token.address,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }
}
