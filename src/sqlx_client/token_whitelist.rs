use crate::models::*;
use crate::sqlx_client::*;

impl SqlxClient {
    pub async fn get_token_whitelist(&self) -> Result<Vec<TokenWhitelistFromDb>, anyhow::Error> {
        let res = sqlx::query_as!(
            TokenWhitelistFromDb,
            r#"SELECT name, address, contract FROM token_whitelist"#
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
                (name, address, contract)
                VALUES ($1, $2, $3)
                RETURNING
                name, address, contract"#,
            root_token.name,
            root_token.address,
            root_token.contract,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(From::from)
    }
}
