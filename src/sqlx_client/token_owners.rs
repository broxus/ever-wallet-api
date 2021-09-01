use crate::models::*;
use crate::sqlx_client::*;

impl SqlxClient {
    pub async fn get_token_owner_by_address(
        &self,
        address: String,
    ) -> Result<TokenOwnerFromDb, anyhow::Error> {
        let res = sqlx::query_as!(
            TokenOwnerFromDb,
            r#"SELECT address, owner_account_workchain_id, owner_account_hex, root_address, code_hash, created_at
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
            r#"INSERT INTO token_owners (address, owner_account_workchain_id, owner_account_hex, root_address, code_hash)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT DO NOTHING"#,
            token_owner.address,
            token_owner.owner_account_workchain_id,
            token_owner.owner_account_hex,
            token_owner.root_address,
            token_owner.code_hash
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_all_token_owners(&self) -> Result<Vec<TokenOwnerFromDb>, anyhow::Error> {
        sqlx::query_as!(
            TokenOwnerFromDb,
            r#"SELECT address, owner_account_workchain_id, owner_account_hex, root_address, code_hash, created_at
        FROM token_owners "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(anyhow::Error::new)
    }
}
