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
}
