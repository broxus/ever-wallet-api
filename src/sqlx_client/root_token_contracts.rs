use anyhow::Result;

use crate::api::requests::RootTokenContractsSearchRequest;
use crate::models::sqlx::{RootContractInfoFromDb, RootTokenContractDb};
use crate::sqlx_client::SqlxClient;

impl SqlxClient {
    pub async fn count_root_token_contracts_info_by_token_substring(
        &self,
        search: &RootTokenContractsSearchRequest,
    ) -> Result<i64, anyhow::Error> {
        if let Some(substring) = search.substring.clone() {
            let substring = urlencoding::decode(&substring)?;
            let substring = format!("{}%", substring.to_uppercase());
            sqlx::query!(
            r#"SELECT COUNT(*)
            FROM root_tokens_contracts
             WHERE code_hash=decode('3BAE4A28A3491AA348AD9B4F21CA642828FCECB0A4945246C5BA7AD4F7F87D04','hex') and token LIKE $1"#,
            substring,
        )
                .fetch_one(&self.pool)
                .await
                .map(|x| x.count.unwrap_or_default())
                .map_err(anyhow::Error::new)
        } else {
            sqlx::query!(
            r#"SELECT COUNT(*)
            FROM root_tokens_contracts
             WHERE code_hash=decode('3BAE4A28A3491AA348AD9B4F21CA642828FCECB0A4945246C5BA7AD4F7F87D04','hex')"#,
        )
                .fetch_one(&self.pool)
                .await
                .map(|x| x.count.unwrap_or_default())
                .map_err(anyhow::Error::new)
        }
    }

    pub async fn post_root_token_contracts_info_search(
        &self,
        search: &RootTokenContractsSearchRequest,
    ) -> Result<Vec<RootContractInfoFromDb>, anyhow::Error> {
        if let Some(substring) = search.substring.clone() {
            let substring = urlencoding::decode(&substring)?;
            let substring = format!("{}%", substring.to_uppercase());
            sqlx::query_as!(
            RootContractInfoFromDb,
            r#"SELECT name, token, owner_address, root_address, code_hash, scale, root_public_key
            FROM root_tokens_contracts
            WHERE code_hash=decode('3BAE4A28A3491AA348AD9B4F21CA642828FCECB0A4945246C5BA7AD4F7F87D04','hex') and token LIKE $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"#,
            substring,
            search.limit,
            search.offset,
        )
                .fetch_all(&self.pool)
                .await
                .map_err(anyhow::Error::new)
        } else {
            sqlx::query_as!(
            RootContractInfoFromDb,
            r#"SELECT name, token, owner_address, root_address, code_hash, scale, root_public_key
            FROM root_tokens_contracts
            WHERE code_hash=decode('3BAE4A28A3491AA348AD9B4F21CA642828FCECB0A4945246C5BA7AD4F7F87D04','hex') ORDER BY created_at DESC LIMIT $1 OFFSET $2"#,
            search.limit,
            search.offset,
        )
                .fetch_all(&self.pool)
                .await
                .map_err(anyhow::Error::new)
        }
    }

    pub async fn get_root_token_contracts_info_by_token_substring(
        &self,
        symbol: String,
    ) -> Result<Vec<RootContractInfoFromDb>, anyhow::Error> {
        let symbol = format!("{}%", symbol.to_uppercase());
        sqlx::query_as!(
            RootContractInfoFromDb,
            r#"SELECT name, token, owner_address, root_address, code_hash, scale, root_public_key
            FROM root_tokens_contracts
             WHERE code_hash=decode('3BAE4A28A3491AA348AD9B4F21CA642828FCECB0A4945246C5BA7AD4F7F87D04','hex') and token LIKE $1"#,
            symbol
        )
        .fetch_all(&self.pool)
        .await
        .map_err(anyhow::Error::new)
    }

    pub async fn get_root_token_contract_by_address(
        &self,
        root_address: String,
    ) -> Result<RootTokenContractDb> {
        sqlx::query_as!(
            RootTokenContractDb,
            r#"SELECT root_address, code_hash, state, token, name, scale, created_at, owner_address, root_public_key
            FROM root_tokens_contracts
             WHERE root_address = $1"#,
            root_address
        )
        .fetch_one(&self.pool)
        .await
        .map_err(anyhow::Error::new)
    }

    pub async fn new_root_token_contract(&self, token_owner: &RootTokenContractDb) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO root_tokens_contracts (root_address,code_hash,state,token,name, scale,owner_address, root_public_key)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)  ON CONFLICT DO NOTHING
            "#,
            token_owner.root_address,
            token_owner.code_hash,
            token_owner.state,
            token_owner.token,
            token_owner.name,
            token_owner.scale,
            token_owner.owner_address,
            token_owner.root_public_key
        )
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_all_root_token_contracts(&self) -> Result<Vec<RootTokenContractDb>> {
        let res = sqlx::query_as!(
            RootTokenContractDb,
            r#"SELECT root_address,code_hash,state,token,name, scale,owner_address,root_public_key,created_at
        FROM root_tokens_contracts"#,
        )
            .fetch_all(&self.pool)
            .await;
        Ok(res?)
    }

    pub async fn get_root_contract_by_address(
        &self,
        address: &str,
    ) -> Result<RootContractInfoFromDb, anyhow::Error> {
        sqlx::query_as!(RootContractInfoFromDb,
            "SELECT name, token, owner_address, root_address, code_hash, scale, root_public_key from trading_ton_wallet_api_rs.public.root_tokens_contracts
            WHERE root_address= $1 and code_hash=decode('3BAE4A28A3491AA348AD9B4F21CA642828FCECB0A4945246C5BA7AD4F7F87D04','hex')",
        address).fetch_one(&self.pool).await.map_err(anyhow::Error::new)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn daw() {
        let block = ton_block::Block::default();
        println!("{}", std::mem::size_of_val(&block));
    }
}
