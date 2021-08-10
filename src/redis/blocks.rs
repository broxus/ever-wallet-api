use crate::prelude::RedisPooledConnection;
use ton_api::ton::ton_node::blockid::BlockId;

pub trait BlocksRepo {
    fn get(&mut self) -> Result<Option<BlockId>, anyhow::Error>;
    fn set(&mut self, block: BlockId) -> Result<(), anyhow::Error>;
}

#[derive(derive_more::Constructor)]
pub struct BlocksRepoImpl<'a> {
    redis_conn: &'a mut RedisPooledConnection,
}

impl<'a> BlocksRepo for BlocksRepoImpl<'a> {
    fn get(&mut self) -> Result<Option<BlockId>, anyhow::Error> {
        let block: Option<String> = redis::cmd("GET")
            .arg(create_key())
            .query(&mut **self.redis_conn)?;

        Ok(block.and_then(|block| serde_json::from_str(&block).ok()))
    }

    fn set(&mut self, block: BlockId) -> Result<(), anyhow::Error> {
        let serialized = serde_json::to_string(&block).unwrap();

        redis::cmd("SET")
            .arg(create_key())
            .arg(serialized)
            .query(&mut **self.redis_conn)?;

        Ok(())
    }
}

#[inline]
fn create_key() -> String {
    "ton_wallet_api_rs_last_block".to_string()
}
