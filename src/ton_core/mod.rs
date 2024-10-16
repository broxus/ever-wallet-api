use std::fs;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use nekoton::transport::models::*;
use nekoton_abi::*;
use parking_lot::Mutex;
use tokio::sync::{mpsc, oneshot};
use ton_block::{GetRepresentationHash, MsgAddressInt, Serializable};
use ton_indexer::GlobalConfig;
use ton_types::UInt256;

use self::monitoring::*;
use self::ton_subscriber::*;
use crate::models::*;
use crate::sqlx_client::*;
use crate::utils::*;

mod monitoring;
mod settings;
mod ton_subscriber;

pub use self::settings::*;

pub struct TonCore {
    pub context: Arc<TonCoreContext>,
    pub full_state: Mutex<Arc<FullState>>,
    pub ton_transaction: Mutex<Arc<TonTransaction>>,
    pub token_transaction: Mutex<Arc<TokenTransaction>>,
}

impl TonCore {
    pub async fn new(
        node_config: NodeConfig,
        global_config: ton_indexer::GlobalConfig,
        sqlx_client: SqlxClient,
        owners_cache: OwnersCache,
        ton_transaction_producer: TonTransactionTx,
        token_transaction_producer: TokenTransactionTx,
    ) -> Result<Arc<Self>> {
        let context =
            TonCoreContext::new(node_config, global_config, sqlx_client, owners_cache).await?;

        let full_state = FullState::new(context.clone()).await?;

        let ton_transaction =
            TonTransaction::new(context.clone(), ton_transaction_producer).await?;

        let token_transaction =
            TokenTransaction::new(context.clone(), token_transaction_producer).await?;

        Ok(Arc::new(Self {
            context,
            full_state: Mutex::new(full_state),
            ton_transaction: Mutex::new(ton_transaction),
            token_transaction: Mutex::new(token_transaction),
        }))
    }

    pub async fn start(&self) -> Result<()> {
        // Sync node and subscribers
        self.context.start().await?;

        // Done
        Ok(())
    }

    pub fn add_ton_account_subscription<I>(&self, accounts: I)
    where
        I: IntoIterator<Item = UInt256>,
    {
        self.ton_transaction
            .lock()
            .add_account_subscription(accounts);
    }

    pub fn get_contract_state(&self, account: &UInt256) -> Result<ExistingContract> {
        self.context.get_contract_state(account)
    }

    pub async fn send_ton_message(
        &self,
        account: &UInt256,
        message: &ton_block::Message,
        expire_at: u32,
    ) -> Result<MessageStatus> {
        self.context
            .send_ton_message(account, message, expire_at)
            .await
    }

    pub fn add_pending_message(
        &self,
        account: UInt256,
        message_hash: UInt256,
        expire_at: u32,
    ) -> Result<oneshot::Receiver<MessageStatus>> {
        self.context
            .add_pending_message(account, message_hash, expire_at)
    }

    pub fn current_utime(&self) -> u32 {
        self.context.ton_subscriber.current_utime()
    }

    pub fn signature_id(&self) -> Option<i32> {
        self.context.ton_subscriber.signature_id()
    }
}

pub struct TonCoreContext {
    pub sqlx_client: SqlxClient,
    pub owners_cache: OwnersCache,
    pub messages_queue: Arc<PendingMessagesQueue>,
    pub ton_subscriber: Arc<TonSubscriber>,
    pub ton_engine: Arc<ton_indexer::Engine>,
}

impl Drop for TonCoreContext {
    fn drop(&mut self) {
        self.ton_engine.shutdown();
    }
}

impl TonCoreContext {
    async fn new(
        node_config: NodeConfig,
        global_config: GlobalConfig,
        sqlx_client: SqlxClient,
        owners_cache: OwnersCache,
    ) -> Result<Arc<Self>> {
        let recover_indexer = node_config.recover_indexer;

        let node_config = node_config
            .build_indexer_config()
            .await
            .context("Failed to build node config")?;

        if recover_indexer {
            if let Err(e) = fs::remove_dir_all(&node_config.rocks_db_path) {
                log::error!("Error on remove rocks db - {}", e.to_string());
            }
            if let Err(e) = fs::remove_dir_all(&node_config.file_db_path) {
                log::error!("Error on remove file db - {}", e.to_string());
            }
        }

        let messages_queue = PendingMessagesQueue::new(512);

        let ton_subscriber = TonSubscriber::new(messages_queue.clone());

        let ton_engine = ton_indexer::Engine::new(
            node_config,
            global_config,
            ton_subscriber.clone() as Arc<dyn ton_indexer::Subscriber>,
        )
        .await?;

        Ok(Arc::new(Self {
            sqlx_client,
            owners_cache,
            messages_queue,
            ton_subscriber,
            ton_engine,
        }))
    }

    async fn start(&self) -> Result<()> {
        self.ton_engine.start().await?;

        // Load last states if exists
        let block_ids = self.sqlx_client.get_last_key_blocks().await?;
        for block_id in block_ids {
            let block_id = ton_block::BlockIdExt::from_str(&block_id.block_id)?;
            if let Ok(state) = self.ton_engine.load_state(&block_id).await {
                self.ton_subscriber
                    .update_shards_accounts_cache(block_id.shard_id, state)?;
            }
        }

        self.ton_subscriber.start(&self.ton_engine).await?;
        Ok(())
    }

    fn get_contract_state(&self, account: &UInt256) -> Result<ExistingContract> {
        match self
            .ton_subscriber
            .get_contract_state(account)
            .and_then(make_existing_contract)?
        {
            Some(contract) => Ok(contract),
            None => Err(TonCoreError::AccountNotExist(account.to_hex_string()).into()),
        }
    }

    async fn send_ton_message(
        &self,
        account: &UInt256,
        message: &ton_block::Message,
        expire_at: u32,
    ) -> Result<MessageStatus> {
        let to = match message.header() {
            ton_block::CommonMsgInfo::ExtInMsgInfo(header) => header.dst.workchain_id(),
            _ => return Err(TonCoreError::ExternalTonMessageExpected.into()),
        };

        let cells = message.write_to_new_cell()?.into_cell()?;
        let serialized = ton_types::serialize_toc(&cells)?;

        let rx = self
            .messages_queue
            .add_message(*account, cells.repr_hash(), expire_at)?;

        self.ton_engine
            .broadcast_external_message(to, &serialized)?;

        let status = rx.await?;
        Ok(status)
    }

    fn add_pending_message(
        &self,
        account: UInt256,
        message_hash: UInt256,
        expire_at: u32,
    ) -> Result<oneshot::Receiver<MessageStatus>> {
        self.messages_queue
            .add_message(account, message_hash, expire_at)
    }
}

#[derive(Debug)]
pub enum CaughtTonTransaction {
    Create(CreateReceiveTransaction),
    UpdateSent(UpdateSentTransaction),
}

pub type TonTransactionTx =
    mpsc::UnboundedSender<(CaughtTonTransaction, HandleTransactionStatusTx)>;
pub type TonTransactionRx =
    mpsc::UnboundedReceiver<(CaughtTonTransaction, HandleTransactionStatusTx)>;

pub type TokenTransactionTx =
    mpsc::UnboundedSender<(CreateTokenTransaction, HandleTransactionStatusTx)>;
pub type TokenTransactionRx =
    mpsc::UnboundedReceiver<(CreateTokenTransaction, HandleTransactionStatusTx)>;

pub type FullStateTx = mpsc::UnboundedSender<(ShardAccounts, HandleTransactionStatusTx)>;
pub type FullStateRx = mpsc::UnboundedReceiver<(ShardAccounts, HandleTransactionStatusTx)>;

#[derive(thiserror::Error, Debug)]
enum TonCoreError {
    #[error("External ton message expected")]
    ExternalTonMessageExpected,
    #[error("Account `{0}` not exist")]
    AccountNotExist(String),
    #[error("Root token `{0}` not included in the whitelist")]
    InvalidRootToken(String),
    #[error("Invalid contract address")]
    InvalidContractAddress,
}
