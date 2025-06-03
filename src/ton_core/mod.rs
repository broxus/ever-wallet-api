use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use everscale_types::boc::BocRepr;
use everscale_types::cell::CellBuilder;
use everscale_types::cell::HashBytes;
use nekoton::transport::models::*;
use nekoton_abi::*;
use parking_lot::Mutex;
use tokio::sync::{mpsc, oneshot}; 
use everscale_types::models::*;
use tycho_core::blockchain_rpc::BlockchainRpcClient;
use tycho_storage::KeyBlocksDirection;
use tycho_storage::Storage;

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
        sqlx_client: SqlxClient,
        owners_cache: OwnersCache,
        ton_transaction_producer: TonTransactionTx,
        token_transaction_producer: TokenTransactionTx,
    ) -> Result<Arc<Self>> {
        let context =
            TonCoreContext::new( sqlx_client, owners_cache).await?;

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
        I: IntoIterator<Item = HashBytes>,
    {
        self.ton_transaction
            .lock()
            .add_account_subscription(accounts);
    }

    pub fn get_contract_state(&self, account: &HashBytes) -> Result<ExistingContract> {
        self.context.get_contract_state(account)
    }

    pub async fn send_ton_message(
        &self,
        account: &HashBytes,
        message: &Message<'_>,
        expire_at: u32,
    ) -> Result<MessageStatus> {
        self.context
            .send_ton_message(account, message, expire_at)
            .await
    }

    pub fn add_pending_message(
        &self,
        account: HashBytes,
        message_hash: HashBytes,
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
    pub storage: Storage,
    pub blockchain_rpc_client: BlockchainRpcClient,
}

impl TonCoreContext {
    async fn new(
        sqlx_client: SqlxClient,
        owners_cache: OwnersCache,
        storage: Storage,
        blockchain_rpc_client: BlockchainRpcClient,
    ) -> Result<Arc<Self>> {
        let messages_queue = PendingMessagesQueue::new(512);

        let ton_subscriber = TonSubscriber::new(messages_queue.clone());

        Ok(Arc::new(Self {
            sqlx_client,
            owners_cache,
            messages_queue,
            ton_subscriber,
            storage,
            blockchain_rpc_client,
        }))
    }

    async fn start(&self) -> Result<()> {
        // Load last states if exists
        let block_ids = self.sqlx_client.get_last_key_blocks().await?;
        for block_id in block_ids {
            let block_id = BlockId::from_str(&block_id.block_id)?;
            if let Ok(state) = self.storage.shard_state_storage().load_state(&block_id).await {
                self.ton_subscriber
                    .update_shards_accounts_cache(block_id.shard, state)?;
            }
        }

         let block_handle_storage = self.storage.block_handle_storage();

        // Find the key block with max seqno which was preduced not later than `utime`
        let handle = 'last_key_block: {
            let iter = block_handle_storage.key_blocks_iterator(KeyBlocksDirection::Backward);
            for key_block_id in iter {
                let handle = block_handle_storage
                    .load_handle(&key_block_id)
                    .with_context(|| format!("key block not found: {key_block_id}"))?;
                    break 'last_key_block Some(handle);
            }
            None
        };

        // Load block 
        let block_stuff = match handle  {
            Some(handle) => Some(self.storage.block_storage().load_block_data(&handle).await?.block()),
            None => None
        };

        self.ton_subscriber.start(block_stuff).await?;

        Ok(())
    }

    fn get_contract_state(&self, account: &HashBytes) -> Result<ExistingContract> {
        match self
            .ton_subscriber
            .get_contract_state(account)
            .and_then(make_existing_contract)?
        {
            Some(contract) => Ok(contract),
            None => Err(TonCoreError::AccountNotExist(account.to_string()).into()),
        }
    }

    async fn send_ton_message(
        &self,
        account: &HashBytes,
        message: &Message<'_>,
        expire_at: u32,
    ) -> Result<MessageStatus> {
        match &message.info {
            MsgInfo::ExtIn(header) => header.dst.workchain(),
            _ => return Err(TonCoreError::ExternalTonMessageExpected.into()),
        };

        let rx = self
            .messages_queue
            .add_message(*account, *CellBuilder::build_from(message)?.repr_hash(), expire_at)?;

        let serialized = BocRepr::encode(message)
            ?;

        self.blockchain_rpc_client
            .broadcast_external_message(&*serialized).await;

        let status = rx.await?;
        Ok(status)
    }

    fn add_pending_message(
        &self,
        account: HashBytes,
        message_hash: HashBytes,
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
