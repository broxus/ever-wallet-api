use sqlx::PgPool;

mod bad_transactions;
mod balances;
mod keys;
mod raw_transactions;
mod root_token_contracts;
mod token_owners;
mod transactions;

#[derive(Clone)]
pub struct SqlxClient {
    pool: PgPool,
}

impl SqlxClient {
    pub fn new(pool: PgPool) -> SqlxClient {
        SqlxClient { pool }
    }
}

#[cfg(test)]
mod test {}
