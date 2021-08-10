use sqlx::postgres::PgArguments;
use sqlx::Arguments;
use sqlx::Row;

use crate::models::balances::BalancesSearch;
use crate::models::balances_ordering::BalancesOrdering;
use crate::models::sqlx::BalanceFromDb;
use crate::sqlx_client::SqlxClient;
use bigdecimal::BigDecimal;
use std::collections::HashMap;

impl SqlxClient {
    pub async fn get_total_supply_by_root_addresses(
        &self,
        root_addresses: Vec<String>,
    ) -> Result<HashMap<String, BigDecimal>, anyhow::Error> {
        let res = sqlx::query!(r#"SELECT root_address, SUM(amount) FROM balances WHERE root_address = ANY($1) GROUP BY root_address"#,
        &root_addresses).fetch_all(&self.pool).await?;
        let res = res
            .into_iter()
            .map(|x| (x.root_address, x.sum.unwrap_or_default()))
            .collect::<HashMap<_, _>>();
        Ok(res)
    }

    pub async fn get_total_supply_by_root_address(
        &self,
        root_address: &str,
    ) -> Result<BigDecimal, anyhow::Error> {
        sqlx::query!(
            r#"SELECT SUM(amount) FROM balances WHERE root_address = $1"#,
            root_address
        )
        .fetch_one(&self.pool)
        .await
        .map(|x| x.sum.unwrap_or_default())
        .map_err(anyhow::Error::new)
    }

    pub async fn find_balances(
        &self,
        input: &BalancesSearch,
    ) -> Result<Vec<BalanceFromDb>, anyhow::Error> {
        let BalancesSearch {
            ordering,
            limit,
            offset,
            ..
        } = input.clone();

        let limit = if limit > 5000 { 5000 } else { limit };

        let (params, mut args, args_len) = query_filter_builder(input);

        let mut query: String = format!(
            "SELECT owner_address, public_key, amount, root_address, token, created_at, block_time \
        FROM balances {} ",
            params
        );

        if let Some(ordering) = ordering {
            let ordering = match ordering {
                BalancesOrdering::CreatedAtAscending => "ORDER BY created_at",
                BalancesOrdering::CreatedAtDescending => "ORDER BY created_at DESC",
                BalancesOrdering::AmountAscending => "ORDER BY amount",
                BalancesOrdering::AmountDescending => "ORDER BY amount DESC",
            };
            query = format!(
                "{} {} OFFSET ${} LIMIT ${}",
                query,
                ordering,
                args_len + 1,
                args_len + 2
            );
        } else {
            query = format!(
                "{} ORDER BY created_at DESC OFFSET ${} LIMIT ${}",
                query,
                args_len + 1,
                args_len + 2
            );
        }
        args.add(offset);
        args.add(limit);

        let balances = sqlx::query_with(&query, args).fetch_all(&self.pool).await?;

        let res = balances
            .iter()
            .map(|x| BalanceFromDb {
                owner_address: x.get(0),
                public_key: x.get(1),
                amount: x.get(2),
                root_address: x.get(3),
                token: x.get(4),
                created_at: x.get(5),
                block_time: x.get(6),
            })
            .collect::<Vec<_>>();
        Ok(res)
    }

    pub async fn count_balances(&self, input: &BalancesSearch) -> Result<i64, anyhow::Error> {
        let (params, args, _args_len) = query_filter_builder(input);

        let query: String = format!("SELECT COUNT(*) FROM balances {} ", params);

        sqlx::query_with(&query, args)
            .fetch_one(&self.pool)
            .await
            .map(|x| x.get::<i64, usize>(0))
            .map_err(anyhow::Error::new)
    }
}

fn query_filter_builder(input: &BalancesSearch) -> (String, PgArguments, i32) {
    let BalancesSearch {
        token,
        root_address,
        owner_address,
        public_key,
        ..
    } = input.clone();
    let mut args = PgArguments::default();
    let mut params = Vec::new();
    let mut args_len = 0;

    if let Some(owner_address) = owner_address {
        params.push(format!(" owner_address = ${} ", args_len + 1,));
        args_len += 1;
        args.add(owner_address);
    }

    if let Some(token) = token {
        params.push(format!("token = ${} ", args_len + 1,));
        args_len += 1;
        args.add(token.to_string());
    }

    if let Some(root_address) = root_address {
        params.push(format!(" root_address = ${} ", args_len + 1,));
        args_len += 1;
        args.add(root_address);
    }

    if let Some(public_key) = public_key {
        params.push(format!(" public_key = ${} ", args_len + 1,));
        args.add(public_key)
    }

    let params = if let Some((first, elements)) = params.split_first() {
        elements.iter().fold(format!("WHERE {}", first), |acc, x| {
            format!("{} AND {}", acc, x)
        })
    } else {
        "".to_string()
    };

    (params, args, args_len)
}
