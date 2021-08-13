use std::sync::Arc;

use anyhow::Result;
use serde::Deserialize;

use self::ton_subscriber::*;

mod ton_subscriber;

pub struct TonIndexer {
    ton_engine: Arc<ton_indexer::Engine>,
    ton_subscriber: Arc<TonSubscriber>,

    initialized: tokio::sync::Mutex<bool>,
}

impl TonIndexer {
    pub async fn new(
        config: IndexerConfig,
        global_config: ton_indexer::GlobalConfig,
    ) -> Result<Arc<Self>> {
        let ton_subscriber = TonSubscriber::new();

        let ton_engine = ton_indexer::Engine::new(
            config.ton_indexer,
            global_config,
            vec![ton_subscriber.clone() as Arc<dyn ton_indexer::Subscriber>],
        )
        .await?;

        let engine = Arc::new(Self {
            ton_engine,
            ton_subscriber,
            initialized: Default::default(),
        });

        Ok(engine)
    }

    pub async fn start(&self) -> Result<()> {
        let mut initialized = self.initialized.lock().await;
        if *initialized {
            return Err(EngineError::AlreadyInitialized.into());
        }

        self.ton_engine.start().await?;
        self.ton_subscriber.start().await?;

        *initialized = true;
        Ok(())
    }
}

#[derive(Deserialize, Clone)]
pub struct IndexerConfig {
    pub ton_indexer: ton_indexer::NodeConfig,
}

#[derive(thiserror::Error, Debug)]
enum EngineError {
    #[error("Already initialized")]
    AlreadyInitialized,
}
