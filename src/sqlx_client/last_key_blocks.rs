use anyhow::Result;

use crate::models::*;
use crate::sqlx_client::*;

impl SqlxClient {
    pub async fn create_last_key_block(&self, block_id: &str) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO last_key_blocks (block_id) VALUES ($1)"#,
            block_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_last_key_blocks(&self) -> Result<Vec<LastKeyBlock>> {
        let res = sqlx::query_as!(LastKeyBlock, r#"SELECT block_id FROM last_key_blocks"#)
            .fetch_all(&self.pool)
            .await?;

        Ok(res)
    }
}
