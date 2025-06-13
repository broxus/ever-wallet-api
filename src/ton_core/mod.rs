use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use everscale_types::boc::Boc;
use everscale_types::cell::HashBytes;
use everscale_types::cell::Lazy;
use everscale_types::cell::Load;
use everscale_types::models::*;
use nekoton::transport::models::*;
use nekoton_abi::*;
use nekoton_utils::Clock;
use nekoton_utils::SimpleClock;
use parking_lot::Mutex;
use tokio::sync::{mpsc, oneshot};
use ton_block::Serializable;
use ton_types::UInt256;
use tycho_core::blockchain_rpc::BlockchainRpcClient;
use tycho_executor::ExecutorParams;
use tycho_executor::ParsedConfig;
use tycho_storage::Storage;

use self::monitoring::*;
use self::ton_subscriber::*;
use crate::models::*;
use crate::sqlx_client::*;
use crate::utils::*;

mod monitoring;
mod settings;
mod ton_subscriber;

pub struct TonCore {
    pub context: Arc<TonCoreContext>,
    pub ton_transaction: Mutex<Arc<TonTransaction>>,
    pub token_transaction: Mutex<Arc<TokenTransaction>>,
}

impl TonCore {
    pub async fn new(
        sqlx_client: SqlxClient,
        owners_cache: OwnersCache,
        ton_transaction_producer: TonTransactionTx,
        token_transaction_producer: TokenTransactionTx,
        storage: Storage,
        blockchain_rpc_client: BlockchainRpcClient,
    ) -> Result<Arc<Self>> {
        let context =
            TonCoreContext::new(sqlx_client, owners_cache, storage, blockchain_rpc_client).await?;

        let ton_transaction =
            TonTransaction::new(context.clone(), ton_transaction_producer).await?;

        let token_transaction =
            TokenTransaction::new(context.clone(), token_transaction_producer).await?;

        Ok(Arc::new(Self {
            context,
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
            if let Ok(state) = self
                .storage
                .shard_state_storage()
                .load_state(&block_id)
                .await
            {
                self.ton_subscriber
                    .update_shards_accounts_cache(block_id.shard, state)?;
            }
        }

        // Load last key block
        let block_stuff = match self.storage.block_handle_storage().find_last_key_block() {
            Some(handle) => {
                let block_stuff = self
                    .storage
                    .block_storage()
                    .load_block_data(&handle)
                    .await?;
                Some(block_stuff)
            }
            None => None,
        };

        self.ton_subscriber.start(block_stuff).await?;

        Ok(())
    }

    fn get_contract_state(&self, account: &UInt256) -> Result<ExistingContract> {
        let account = HashBytes::from_slice(account.as_slice());
        match self
            .ton_subscriber
            .get_contract_state(&account)
            .and_then(make_existing_contract)?
        {
            Some(contract) => Ok(contract),
            None => Err(TonCoreError::AccountNotExist(account.to_string()).into()),
        }
    }

    async fn send_ton_message(
        &self,
        account: &UInt256,
        message: &ton_block::Message,
        expire_at: u32,
    ) -> Result<MessageStatus> {
        match message.header() {
            ton_block::CommonMsgInfo::ExtInMsgInfo(header) => header.dst.workchain_id(),
            _ => return Err(TonCoreError::ExternalTonMessageExpected.into()),
        };

        let cells = message.write_to_new_cell()?.into_cell()?;
        let serialized = ton_types::serialize_toc(&cells)?;

        let message_base64: String = base64::encode(&serialized);

        tracing::info!("Sending message {} to account {}", message_base64, account.to_string());

        let rx = self.messages_queue.add_message(
            HashBytes::from_slice(account.as_slice()),
            HashBytes::from_slice(cells.repr_hash().as_slice()),
            expire_at,
        )?;
        
        match self.send_local(account, &*message_base64, expire_at){
            Ok(None) =>
            {
                tracing::info!("transaction send local - no account ");
            },
            Ok(Some(t)) =>
            {
                tracing::info!("transaction send local ok - {:?}, info - {:?}", t, t.load_info()?);
            },
            Err(e) => {
                tracing::error!("transaction send local error: {:?}", e);
            },
        }

        self.blockchain_rpc_client
            .broadcast_external_message(&serialized)
            .await;

        let status = rx.await?;
        Ok(status)
    }

    fn send_local(&self, account: &UInt256, message_base64: &str, expire_at: u32) -> Result<Option<Transaction>> {
        let account = HashBytes::from_slice(account.as_slice());

         let account_state = self.ton_subscriber.get_contract_state(&account)?;

        let account_state = match account_state {
            Some(this) => this,
            None => return Ok(None),
        };

        let account = everscale_types::models::OptionalAccount::load_from(&mut account_state.data.as_slice()?)?;

        let LastTransactionId::Exact(last_transaction_id) = account_state.last_transaction_id else {
            return Ok(None);
        }; 

         let shard_account = everscale_types::models::ShardAccount {
            account: Lazy::new(&account).unwrap(),
            last_trans_hash: HashBytes::from_slice(last_transaction_id.hash.as_slice()),
            last_trans_lt: last_transaction_id.lt,
        }; 
         
        let config_cell = Boc::decode_base64("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").unwrap();
        let config = config_cell.parse::<BlockchainConfig>()?;

        let config = ParsedConfig::parse(config, SimpleClock.now_sec_u64() as u32)?;

         let cell = Boc::decode_base64(message_base64)?;
         let mut cs = cell.as_slice()?;
         let message = OwnedMessage::load_from(&mut cs)?;

         let is_external = !matches!(message.ty(), MsgType::Int);

        let optional = shard_account.load_account()?;
        let Some(account) = optional else {
            return Err(anyhow::anyhow!("account not found"));
        };
        let address = account.address.as_std().unwrap();

        let executor_params = ExecutorParams{
            block_unixtime: expire_at - 10,
            ..Default::default()
        };
        let executor = tycho_executor::Executor::new(&executor_params, &config);
        let uncommited = executor.begin_ordinary(address, is_external, message, &shard_account)?;
        
        uncommited.build_uncommitted().map(Some).map_err(|e|  anyhow::anyhow!(e))
    }
    

    fn add_pending_message(
        &self,
        account: UInt256,
        message_hash: UInt256,
        expire_at: u32,
    ) -> Result<oneshot::Receiver<MessageStatus>> {
        self.messages_queue.add_message(
            HashBytes::from_slice(account.as_slice()),
            HashBytes::from_slice(message_hash.as_slice()),
            expire_at,
        )
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
