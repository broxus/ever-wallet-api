use std::sync::Arc;

use anyhow::{Context, Result};
use nekoton::transport::models::*;
use nekoton_abi::*;
use parking_lot::Mutex;
use tokio::sync::{mpsc, oneshot};
use ton_block::{GetRepresentationHash, MsgAddressInt, Serializable};
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
    context: Arc<TonCoreContext>,
    ton_transaction: Mutex<Option<Arc<TonTransaction>>>,
    token_transaction: Mutex<Option<Arc<TokenTransaction>>>,
}

impl TonCore {
    pub async fn new(
        node_config: NodeConfig,
        global_config: ton_indexer::GlobalConfig,
        sqlx_client: SqlxClient,
        owners_cache: OwnersCache,
        ton_transaction_producer: CaughtTonTransactionTx,
        token_transaction_producer: CaughtTokenTransactionTx,
    ) -> Result<Arc<Self>> {
        let context =
            TonCoreContext::new(node_config, global_config, sqlx_client, owners_cache).await?;

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

    pub fn init_token_subscription(&self) {
        if let Some(token_transaction) = &*self.token_transaction.lock() {
            token_transaction.init_token_subscription();
        }
    }

    pub fn get_contract_state(&self, account: &UInt256) -> Result<ExistingContract> {
        self.context.get_contract_state(account)
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
        node_config: NodeConfig,
        global_config: ton_indexer::GlobalConfig,
        sqlx_client: SqlxClient,
        owners_cache: OwnersCache,
    ) -> Result<Arc<Self>> {
        let node_config = node_config
            .build_indexer_config()
            .await
            .context("Failed to build node config")?;

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

    fn get_contract_state(&self, account: &UInt256) -> Result<ExistingContract> {
        match self.ton_subscriber.get_contract_state(account)? {
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

        log::info!(
            "Broadcast: now - {}; current - {}; expire_at - {}",
            chrono::Utc::now().timestamp(),
            self.ton_subscriber.current_utime(),
            expire_at
        );

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

#[derive(Debug)]
pub enum CaughtTonTransaction {
    Create(CreateReceiveTransaction),
    UpdateSent(UpdateSentTransaction),
}

pub type CaughtTonTransactionTx =
    mpsc::UnboundedSender<(CaughtTonTransaction, HandleTransactionStatusTx)>;
pub type CaughtTonTransactionRx =
    mpsc::UnboundedReceiver<(CaughtTonTransaction, HandleTransactionStatusTx)>;

pub type CaughtTokenTransactionTx =
    mpsc::UnboundedSender<(CreateTokenTransaction, HandleTransactionStatusTx)>;
pub type CaughtTokenTransactionRx =
    mpsc::UnboundedReceiver<(CreateTokenTransaction, HandleTransactionStatusTx)>;

#[derive(thiserror::Error, Debug)]
enum TonCoreError {
    #[error("External ton message expected")]
    ExternalTonMessageExpected,
    #[error("Account `{0}` not exist")]
    AccountNotExist(String),
    #[error("Root token `{0}` is not included in the whitelist")]
    InvalidRootToken(String),
}
