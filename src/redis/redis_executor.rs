use crate::prelude::*;

#[derive(Clone, derive_more::Constructor)]
pub struct RedisExecutorImpl {
    redis_pool: RedisPool,
}

impl RedisExecutorImpl {
    pub fn get_connection(&self) -> Result<RedisPooledConnection, ServiceError> {
        self.redis_pool
            .get()
            .map_err(|e| ServiceError::Other(Box::new(e).into()))
    }
}
