use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use nekoton::transport::models::*;
use nekoton_abi::*;
use parking_lot::Mutex;
use serde::Deserialize;
use tokio::sync::{mpsc, oneshot};
use ton_block::{GetRepresentationHash, MsgAddressInt, Serializable};
use ton_types::UInt256;

use self::monitoring::*;
use self::settings::*;
use self::ton_contracts::*;
use self::ton_subscriber::*;
use crate::models::*;
use crate::sqlx_client::*;
use crate::utils::*;

mod monitoring;
mod settings;
mod ton_contracts;
mod ton_subscriber;

pub struct TonCore {
    context: Arc<TonCoreContext>,
    ton_transaction: Mutex<Option<Arc<TonTransaction>>>,
    token_transaction: Mutex<Option<Arc<TokenTransaction>>>,
}

impl TonCore {
    pub async fn new(
        config: TonCoreConfig,
        global_config: ton_indexer::GlobalConfig,
        sqlx_client: SqlxClient,
        owners_cache: OwnersCache,
        ton_transaction_producer: CaughtTonTransactionTx,
        token_transaction_producer: CaughtTokenTransactionTx,
    ) -> Result<Arc<Self>> {
        let context = TonCoreContext::new(config, global_config, sqlx_client, owners_cache).await?;

        let ton_transaction =
            TonTransaction::new(context.clone(), ton_transaction_producer).await?;

        let token_transaction =
            TokenTransaction::new(context.clone(), token_transaction_producer).await?;

        Ok(Arc::new(Self {
            context,
            ton_transaction: Mutex::new(Some(ton_transaction)),
            token_transaction: Mutex::new(Some(token_transaction)),
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
        if let Some(ton_transaction) = &*self.ton_transaction.lock() {
            ton_transaction.add_account_subscription(accounts);
        }
    }

    pub fn add_token_account_subscription<I>(&self, accounts: I)
    where
        I: IntoIterator<Item = UInt256>,
    {
        if let Some(token_transaction) = &*self.token_transaction.lock() {
            token_transaction.add_account_subscription(accounts);
        }
    }

    pub async fn get_contract_state(&self, account: UInt256) -> Result<ExistingContract> {
        self.context.get_contract_state(account).await
    }

    pub async fn send_ton_message(
        &self,
        account: &ton_types::UInt256,
        message: &ton_block::Message,
        expire_at: u32,
    ) -> Result<MessageStatus> {
        self.context
            .send_ton_message(account, message, expire_at)
            .await
    }

    pub fn current_utime(&self) -> u32 {
        self.context.ton_subscriber.current_utime()
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
}

pub struct TonCoreContext {
    pub sqlx_client: SqlxClient,
    pub owners_cache: OwnersCache,
    pub messages_queue: Arc<PendingMessagesQueue>,
    pub ton_subscriber: Arc<TonSubscriber>,
    pub ton_engine: Arc<ton_indexer::Engine>,
}

impl TonCoreContext {
    async fn new(
        config: TonCoreConfig,
        global_config: ton_indexer::GlobalConfig,
        sqlx_client: SqlxClient,
        owners_cache: OwnersCache,
    ) -> Result<Arc<Self>> {
        let node_config = get_node_config(&config).await?;

        let messages_queue = PendingMessagesQueue::new(1000);

        let ton_subscriber = TonSubscriber::new(messages_queue.clone());
        let ton_engine = ton_indexer::Engine::new(
            node_config,
            global_config,
            vec![ton_subscriber.clone() as Arc<dyn ton_indexer::Subscriber>],
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
        self.ton_subscriber.start().await?;
        Ok(())
    }

    async fn get_contract_state(&self, account: UInt256) -> Result<ExistingContract> {
        match self.ton_subscriber.get_contract_state(account).await? {
            Some(contract) => Ok(contract),
            None => Err(TonCoreError::AccountNotExist(account.to_hex_string()).into()),
        }
    }

    async fn send_ton_message(
        &self,
        account: &ton_types::UInt256,
        message: &ton_block::Message,
        expire_at: u32,
    ) -> Result<MessageStatus> {
        let to = match message.header() {
            ton_block::CommonMsgInfo::ExtInMsgInfo(header) => {
                ton_block::AccountIdPrefixFull::prefix(&header.dst)?
            }
            _ => return Err(TonCoreError::ExternalTonMessageExpected.into()),
        };

        let cells = message.write_to_new_cell()?.into();
        let serialized = ton_types::serialize_toc(&cells)?;

        let rx = self
            .messages_queue
            .add_message(*account, cells.repr_hash(), expire_at)?;

        self.ton_engine
            .broadcast_external_message(&to, &serialized)
            .await?;

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

pub enum CaughtTonTransaction {
    Create(CreateReceiveTransaction),
    UpdateSent(UpdateSentTransaction),
}

pub type CaughtTonTransactionTx = mpsc::UnboundedSender<CaughtTonTransaction>;
pub type CaughtTonTransactionRx = mpsc::UnboundedReceiver<CaughtTonTransaction>;

pub type CaughtTokenTransactionTx = mpsc::UnboundedSender<CreateTokenTransaction>;
pub type CaughtTokenTransactionRx = mpsc::UnboundedReceiver<CreateTokenTransaction>;

#[derive(Deserialize, Clone, Debug)]
pub struct TonCoreConfig {
    pub port: u16,
    pub rocks_db_path: PathBuf,
    pub file_db_path: PathBuf,
    pub keys_path: PathBuf,
}

#[derive(thiserror::Error, Debug)]
enum TonCoreError {
    #[error("External ton message expected")]
    ExternalTonMessageExpected,
    #[error("Account `{0}` not exist")]
    AccountNotExist(String),
}
