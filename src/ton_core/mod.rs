use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use nekoton::transport::models::*;
use nekoton_abi::*;
use parking_lot::Mutex;
use serde::Deserialize;
use tokio::sync::mpsc;
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
    ) -> Result<Arc<Self>> {
        let context = TonCoreContext::new(config, global_config, sqlx_client, owners_cache).await?;

        Ok(Arc::new(Self {
            context,
            ton_transaction: Mutex::new(None),
            token_transaction: Mutex::new(None),
        }))
    }

    pub async fn start(
        &self,
        ton_transaction_producer: CaughtTonTransactionTx,
        token_transaction_producer: CaughtTokenTransactionTx,
        root_state_cache: RootStateCache,
    ) -> Result<()> {
        // Sync node and subscribers
        self.context.start().await?;

        let ton_transaction =
            TonTransaction::new(self.context.clone(), ton_transaction_producer).await?;
        ton_transaction.init_subscriptions().await?;
        *self.ton_transaction.lock() = Some(ton_transaction);

        let token_transaction =
            TokenTransaction::new(self.context.clone(), token_transaction_producer).await?;
        token_transaction
            .init_subscriptions(root_state_cache)
            .await?;
        *self.token_transaction.lock() = Some(token_transaction);

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

    pub fn get_current_utime(&self) -> u32 {
        self.context.ton_subscriber.get_current_utime()
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
            RawContractState::Exists(contract) => Ok(contract),
            RawContractState::NotExists => {
                Err(TonCoreError::AccountNotExist(account.to_hex_string()).into())
            }
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

    /*async fn load_unprocessed_transactions(&self) -> Result<()> {
        let transactions: Vec<TransactionDb> = self
            .sqlx_client
            .get_all_transactions_by_status(TonTransactionStatus::New)
            .await?;

        for transaction in transactions {
            let account = UInt256::from_be_bytes(&hex::decode(transaction.account_hex)?);
            let message_hash = UInt256::from_be_bytes(&hex::decode(transaction.message_hash)?);
            let expire_at = transaction.created_at.timestamp() as u32 + DEFAULT_EXPIRATION_TIMEOUT;
            self.messages_queue
                .add_message(account, message_hash, expire_at)?;
        }

        Ok(())
    }*/
}

/// Generic listener for transactions
struct AccountObserver<T>(AccountEventsTx<T>);

impl<T> AccountObserver<T> {
    fn new(tx: AccountEventsTx<T>) -> Arc<Self> {
        Arc::new(Self(tx))
    }
}

impl<T> TransactionsSubscription for AccountObserver<T>
where
    T: ReadFromTransaction + std::fmt::Debug + Send + Sync,
{
    fn handle_transaction(&self, ctx: TxContext<'_>) -> Result<()> {
        let event = T::read_from_transaction(&ctx);

        log::info!(
            "Got transaction on account {}: {:?}",
            ctx.account.to_hex_string(),
            event
        );

        // Send event to event manager if it exist
        if let Some(event) = event {
            self.0.send(event).ok();
        }

        // Done
        Ok(())
    }
}

pub type AccountEventsTx<T> = mpsc::UnboundedSender<T>;

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
