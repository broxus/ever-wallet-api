use crate::models::sqlx::TokenOwnerFromDb;
use crate::sqlx_client::SqlxClient;

impl SqlxClient {
    pub async fn get_token_owner_by_address(
        &self,
        address: String,
    ) -> Result<TokenOwnerFromDb, anyhow::Error> {
        let res = sqlx::query_as!(
            TokenOwnerFromDb,
            r#"SELECT address, owner_address, owner_public_key, root_address, token, code_hash, scale, created_at
        FROM token_owners
        WHERE address = $1"#,
            address
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(res)
    }

    pub async fn new_token_owner(
        &self,
        token_owner: &TokenOwnerFromDb,
    ) -> Result<(), anyhow::Error> {
        sqlx::query!(
            r#"INSERT INTO token_owners (address, owner_address, owner_public_key, root_address, token, code_hash, scale)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT DO NOTHING"#,
            token_owner.address,
            token_owner.owner_address,
            token_owner.owner_public_key,
            token_owner.root_address,
            token_owner.token,
            token_owner.code_hash,
            token_owner.scale
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_all_token_owners(&self) -> Result<Vec<TokenOwnerFromDb>, anyhow::Error> {
        sqlx::query_as!(
            TokenOwnerFromDb,
            r#"SELECT address, owner_address, owner_public_key, root_address, token, code_hash, scale, created_at
        FROM token_owners "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(anyhow::Error::new)
    }
}
