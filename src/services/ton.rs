use anyhow::Error;
use async_trait::async_trait;
use bigdecimal::{BigDecimal, Zero};

use crate::api::requests::RootTokenContractsSearchRequest;
use crate::models::balances::BalancesSearch;
use crate::models::sqlx::{
    BalanceFromDb, RootContractInfoFromDb, RootTokenContractDb, TokenOwnerFromDb, TransactionFromDb,
};
use crate::models::transactions::TransactionsSearch;
use crate::sqlx_client::SqlxClient;

#[async_trait]
pub trait TonService: Send + Sync + 'static {
    async fn search_transactions(
        &self,
        input: &TransactionsSearch,
    ) -> Result<Vec<TransactionFromDb>, anyhow::Error>;
    async fn count_transactions(&self, input: &TransactionsSearch) -> Result<i64, anyhow::Error>;
    async fn search_balances(
        &self,
        input: &BalancesSearch,
    ) -> Result<Vec<BalanceFromDb>, anyhow::Error>;

    async fn count_balances(&self, input: &BalancesSearch) -> Result<i64, anyhow::Error>;
    async fn get_all_tokens(&self) -> Result<Vec<RootTokenContractDb>, anyhow::Error>;
    async fn get_token_owner_by_address(
        &self,
        address: String,
    ) -> Result<TokenOwnerFromDb, anyhow::Error>;
    async fn get_root_token_by_address(
        &self,
        address: String,
    ) -> Result<(RootContractInfoFromDb, BigDecimal), anyhow::Error>;
    async fn get_root_token_by_token_substring(
        &self,
        token: String,
    ) -> Result<Vec<(RootContractInfoFromDb, BigDecimal)>, anyhow::Error>;
    async fn post_root_tokens_by_token_substring(
        &self,
        search: RootTokenContractsSearchRequest,
    ) -> Result<(Vec<(RootContractInfoFromDb, BigDecimal)>, i64), Error>;
}

pub struct TonServiceImpl {
    sqlx_client: SqlxClient,
}

impl TonServiceImpl {
    pub fn new(sqlx_client: SqlxClient) -> Self {
        Self { sqlx_client }
    }
}

#[async_trait]
impl TonService for TonServiceImpl {
    async fn search_transactions(
        &self,
        input: &TransactionsSearch,
    ) -> Result<Vec<TransactionFromDb>, anyhow::Error> {
        self.sqlx_client.find_transactions(input).await
    }
    async fn count_transactions(&self, input: &TransactionsSearch) -> Result<i64, anyhow::Error> {
        self.sqlx_client.count_transactions(input).await
    }

    async fn search_balances(
        &self,
        input: &BalancesSearch,
    ) -> Result<Vec<BalanceFromDb>, anyhow::Error> {
        self.sqlx_client.find_balances(input).await
    }

    async fn count_balances(&self, input: &BalancesSearch) -> Result<i64, Error> {
        self.sqlx_client.count_balances(input).await
    }

    async fn get_all_tokens(&self) -> Result<Vec<RootTokenContractDb>, Error> {
        use crate::ws_indexer::TOKEN_WALLET_CODE_HASH;
        let tokens = self.sqlx_client.get_all_root_token_contracts().await?;
        Ok(tokens
            .into_iter()
            .filter(|x| x.code_hash == TOKEN_WALLET_CODE_HASH)
            .collect())
    }

    async fn get_token_owner_by_address(&self, address: String) -> Result<TokenOwnerFromDb, Error> {
        let address = urlencoding::decode(&address)?;
        self.sqlx_client
            .get_token_owner_by_address(address.to_string())
            .await
    }

    async fn get_root_token_by_address(
        &self,
        address: String,
    ) -> Result<(RootContractInfoFromDb, BigDecimal), Error> {
        let address = urlencoding::decode(&address)?;
        let total_supply = self
            .sqlx_client
            .get_total_supply_by_root_address(&address.to_string())
            .await
            .unwrap_or_default();
        let root_contract = self
            .sqlx_client
            .get_root_contract_by_address(&address.to_string())
            .await?;
        Ok((root_contract, total_supply))
    }

    async fn get_root_token_by_token_substring(
        &self,
        token: String,
    ) -> Result<Vec<(RootContractInfoFromDb, BigDecimal)>, Error> {
        let token = urlencoding::decode(&token)?;
        let res = self
            .sqlx_client
            .get_root_token_contracts_info_by_token_substring(token.to_string())
            .await?;
        let addresses = res
            .iter()
            .map(|x| x.root_address.clone())
            .collect::<Vec<_>>();
        let supplies = self
            .sqlx_client
            .get_total_supply_by_root_addresses(addresses)
            .await?;
        let res = res
            .into_iter()
            .map(|x| {
                (
                    x.clone(),
                    supplies
                        .get(&x.root_address)
                        .unwrap_or(&BigDecimal::zero())
                        .clone(),
                )
            })
            .collect::<Vec<_>>();
        Ok(res)
    }

    async fn post_root_tokens_by_token_substring(
        &self,
        search: RootTokenContractsSearchRequest,
    ) -> Result<(Vec<(RootContractInfoFromDb, BigDecimal)>, i64), Error> {
        let res = self
            .sqlx_client
            .post_root_token_contracts_info_search(&search)
            .await?;
        let total_count = self
            .sqlx_client
            .count_root_token_contracts_info_by_token_substring(&search)
            .await?;
        let addresses = res
            .iter()
            .map(|x| x.root_address.clone())
            .collect::<Vec<_>>();
        let supplies = self
            .sqlx_client
            .get_total_supply_by_root_addresses(addresses)
            .await?;
        let res = res
            .into_iter()
            .map(|x| {
                (
                    x.clone(),
                    supplies
                        .get(&x.root_address)
                        .unwrap_or(&BigDecimal::zero())
                        .clone(),
                )
            })
            .collect::<Vec<_>>();
        Ok((res, total_count))
    }
}

#[cfg(test)]
mod test {
    use bigdecimal::{BigDecimal, ToPrimitive};
    #[test]
    fn test_url() {
        let str = "0%3A5b325f4f364366d9b3fe46cc77f622b013da7a7edf99a3d5d25e5510dca50d13";
        let str2 = urlencoding::decode(str).unwrap().to_string();
        let str3 = "0:5b325f4f364366d9b3fe46cc77f622b013da7a7edf99a3d5d25e5510dca50d13";
        let str4 = urlencoding::decode(str3).unwrap().to_string();
        println!("{}", str2);
        println!("{}", str4);

        let big_decimal = BigDecimal::new(123.into(), 2);
        let lol = big_decimal.to_i64().unwrap();
        println!("{}", lol);
    }
}
