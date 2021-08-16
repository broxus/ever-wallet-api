use sqlx::PgPool;

mod addresses;
mod keys;
mod token_owners;
mod token_transactions;
mod transactions;
mod transactions_events;

#[derive(Clone)]
pub struct SqlxClient {
    pool: PgPool,
}

impl SqlxClient {
    pub fn new(pool: PgPool) -> SqlxClient {
        SqlxClient { pool }
    }
}
