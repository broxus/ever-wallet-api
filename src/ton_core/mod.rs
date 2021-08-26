use std::collections::{hash_map, HashMap};
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use nekoton::core::models::{RootTokenContractDetails, TokenWalletTransaction, TokenWalletVersion};
use nekoton::core::token_wallet::{RootTokenContractState, TokenWalletContractState};
use nekoton::transport::models::RawContractState;
use nekoton_abi::LastTransactionId;
use parking_lot::Mutex;
use serde::Deserialize;
use tokio::sync::mpsc;
use ton_block::{GetRepresentationHash, MsgAddressInt, Serializable};
use ton_types::UInt256;

use self::models::*;
use self::ton_subscriber::*;
use self::transaction_handler::token_transaction::*;
use self::transaction_handler::transaction::*;

use crate::models::owners_cache::*;
use crate::models::token_transactions::*;
use crate::models::transactions::*;

mod models;
mod ton_subscriber;
mod transaction_handler;

pub struct TonCore {
    ton_engine: Arc<ton_indexer::Engine>,
    ton_subscriber: Arc<TonSubscriber>,

    owners_cache: OwnersCache,

    transaction_producer: ReceiveTransactionTx,
    token_transaction_producer: ReceiveTokenTransactionTx,

    transaction_observer: Arc<TransactionObserver>,
    token_transaction_observer: Arc<TokenTransactionObserver>,

    pending_messages: Mutex<HashMap<UInt256, u32>>,

    initialized: tokio::sync::Mutex<bool>,
}

impl TonCore {
    pub async fn new(
        config: TonCoreConfig,
        global_config: ton_indexer::GlobalConfig,
        owners_cache: OwnersCache,
        transaction_producer: ReceiveTransactionTx,
        token_transaction_producer: ReceiveTokenTransactionTx,
    ) -> Result<Arc<Self>> {
        let ton_subscriber = TonSubscriber::new();

        let ton_engine = ton_indexer::Engine::new(
            config.ton_indexer,
            global_config,
            vec![ton_subscriber.clone() as Arc<dyn ton_indexer::Subscriber>],
        )
        .await?;

        let (transaction_tx, transaction_rx) = mpsc::unbounded_channel();
        let (token_transaction_tx, token_transaction_rx) = mpsc::unbounded_channel();

        let engine = Arc::new(Self {
            ton_engine,
            owners_cache,
            ton_subscriber,
            transaction_producer,
            token_transaction_producer,
            transaction_observer: Arc::new(TransactionObserver { tx: transaction_tx }),
            token_transaction_observer: Arc::new(TokenTransactionObserver {
                tx: token_transaction_tx,
            }),
            pending_messages: Mutex::new(HashMap::new()),
            initialized: Default::default(),
        });

        engine.start_listening_transactions(transaction_rx);
        engine.start_listening_token_transactions(token_transaction_rx);

        engine.start_pending_messages_watcher();

        Ok(engine)
    }

    pub async fn start(&self) -> Result<()> {
        let mut initialized = self.initialized.lock().await;
        if *initialized {
            return Err(TonCoreError::AlreadyInitialized.into());
        }

        self.ton_engine.start().await?;
        self.ton_subscriber.start().await?;

        *initialized = true;
        Ok(())
    }

    pub async fn get_ton_address_info(&self, account: UInt256) -> Result<TonAddressInfo> {
        let contract = match self.ton_subscriber.get_contract_state(account).await? {
            RawContractState::Exists(contract) => contract,
            RawContractState::NotExists => {
                return Err(TonCoreError::AccountNotFound(account.to_hex_string()).into())
            }
        };

        let workchain_id = contract.account.addr.workchain_id();
        let hex = contract.account.addr.address().to_hex_string();
        let account_status = contract.account.storage.state;
        let network_balance = contract.account.storage.balance.grams.0;

        let mut last_transaction_hash = None;
        let mut last_transaction_lt = None;
        if let LastTransactionId::Exact(transaction_id) = contract.last_transaction_id {
            last_transaction_hash = Some(transaction_id.hash);
            last_transaction_lt = Some(transaction_id.lt)
        }

        Ok(TonAddressInfo {
            workchain_id,
            hex,
            network_balance,
            account_status,
            last_transaction_lt,
            last_transaction_hash,
        })
    }

    pub async fn get_token_address_info(&self, account: UInt256) -> Result<TokenAddressInfo> {
        let contract = match self.ton_subscriber.get_contract_state(account).await? {
            RawContractState::Exists(contract) => contract,
            RawContractState::NotExists => {
                return Err(TonCoreError::AccountNotFound(account.to_hex_string()).into());
            }
        };

        let token_wallet = TokenWalletContractState(&contract);
        let version = token_wallet.get_version()?;
        let root_address = token_wallet.get_details(version)?.root_address;
        let network_balance = token_wallet.get_balance(version)?;

        let workchain_id = contract.account.addr.workchain_id();
        let hex = contract.account.addr.address().to_hex_string();
        let account_status = contract.account.storage.state;

        let mut last_transaction_hash = None;
        let mut last_transaction_lt = None;
        if let nekoton_abi::LastTransactionId::Exact(transaction_id) = contract.last_transaction_id
        {
            last_transaction_hash = Some(transaction_id.hash);
            last_transaction_lt = Some(transaction_id.lt)
        }

        Ok(TokenAddressInfo {
            workchain_id,
            hex,
            root_address,
            network_balance,
            account_status,
            last_transaction_lt,
            last_transaction_hash,
        })
    }

    pub async fn get_token_address(&self, owner: OwnerInfo) -> Result<MsgAddressInt> {
        let root_account = UInt256::from_be_bytes(&owner.root_address.address().get_bytestring(0));
        let root_contract = match self.ton_subscriber.get_contract_state(root_account).await? {
            RawContractState::Exists(contract) => contract,
            RawContractState::NotExists => {
                return Err(TonCoreError::AccountNotFound(root_account.to_hex_string()).into());
            }
        };

        let state = RootTokenContractState(&root_contract);
        let RootTokenContractDetails { version, .. } = state.guess_details()?;

        state.get_wallet_address(version, &owner.owner_address, None)
    }

    pub async fn send_ton_message(
        &self,
        message: &ton_block::Message,
        expire_at: u32,
    ) -> Result<()> {
        let to = match message.header() {
            ton_block::CommonMsgInfo::ExtInMsgInfo(header) => {
                ton_block::AccountIdPrefixFull::prefix(&header.dst)?
            }
            _ => return Err(TonCoreError::ExternalTonMessageExpected.into()),
        };

        let cells = message.write_to_new_cell()?.into();
        let serialized = ton_types::serialize_toc(&cells)?;

        self.add_pending_message(message, expire_at)?;

        match self
            .ton_engine
            .broadcast_external_message(&to, &serialized)
            .await
        {
            Ok(()) => Ok(()),
            Err(e) => {
                self.cancel_pending_message(message)?;
                Err(e)
            }
        }
    }

    pub fn add_account_subscription<I>(&self, accounts: I)
    where
        I: IntoIterator<Item = UInt256>,
    {
        self.ton_subscriber
            .add_transactions_subscription(accounts, &self.transaction_observer);
    }

    pub fn add_token_account_subscription<I>(&self, accounts: I)
    where
        I: IntoIterator<Item = UInt256>,
    {
        self.ton_subscriber
            .add_transactions_subscription(accounts, &self.token_transaction_observer);
    }

    fn start_listening_transactions(
        self: &Arc<Self>,
        mut rx: mpsc::UnboundedReceiver<TransactionContext>,
    ) {
        let engine = Arc::downgrade(self);

        tokio::spawn(async move {
            while let Some(transaction_ctx) = rx.recv().await {
                let engine = match engine.upgrade() {
                    Some(engine) => engine,
                    None => break,
                };

                log::info!("Transaction context: {:#?}", transaction_ctx);

                if let Some(in_msg) = transaction_ctx
                    .transaction
                    .in_msg
                    .as_ref()
                    .and_then(|data| data.read_struct().ok())
                {
                    match handle_transaction(transaction_ctx).await {
                        Ok(transaction) => {
                            if let Some(transaction) = transaction {
                                engine.transaction_producer.send(transaction).ok();
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to handle received transaction: {}", e);
                        }
                    }
                }
            }

            rx.close();
            while rx.recv().await.is_some() {}
        });
    }

    fn start_listening_token_transactions(
        self: &Arc<Self>,
        mut rx: mpsc::UnboundedReceiver<(TokenTransactionContext, TokenWalletTransaction)>,
    ) {
        let engine = Arc::downgrade(self);

        tokio::spawn(async move {
            while let Some((token_transaction_ctx, parsed_token_transaction)) = rx.recv().await {
                let engine = match engine.upgrade() {
                    Some(engine) => engine,
                    None => break,
                };

                log::info!("Token transaction context: {:#?}", token_transaction_ctx);
                log::info!("Parsed token transaction: {:#?}", parsed_token_transaction);

                match handle_token_transaction(
                    token_transaction_ctx,
                    parsed_token_transaction,
                    &engine.owners_cache,
                )
                .await
                {
                    Ok(transaction) => {
                        engine.token_transaction_producer.send(transaction).ok();
                    }
                    Err(e) => {
                        log::error!("Failed to handle received token transaction: {}", e);
                    }
                }
            }

            rx.close();
            while rx.recv().await.is_some() {}
        });
    }

    fn add_pending_message(&self, message: &ton_block::Message, expire_at: u32) -> Result<()> {
        let mut msg_cache = self.pending_messages.lock();
        let msg_hash = message.serialize()?.repr_hash();
        match msg_cache.entry(msg_hash) {
            hash_map::Entry::Vacant(entry) => {
                entry.insert(expire_at);
            }
            hash_map::Entry::Occupied(_) => {
                return Err(TonCoreError::PendingMessageExist(msg_hash.to_hex_string()).into());
            }
        };
        Ok(())
    }

    fn cancel_pending_message(&self, message: &ton_block::Message) -> Result<()> {
        let mut msg_cache = self.pending_messages.lock();
        let msg_hash = message.serialize()?.repr_hash();
        if msg_cache
            .remove(&message.serialize()?.repr_hash())
            .is_none()
        {
            return Err(TonCoreError::PendingMessageNotExist(msg_hash.to_hex_string()).into());
        }

        Ok(())
    }

    fn start_pending_messages_watcher(self: &Arc<Self>) {
        let engine = Arc::downgrade(self);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(1000));

            while let Some(engine) = engine.upgrade() {
                interval.tick().await;

                let now = Utc::now().timestamp() as u32;

                let mut msg_cache = engine.pending_messages.lock();
                msg_cache.retain(|msg_hash, expire_at| {
                    let expired = now > *expire_at;
                    if expired {
                        // TODO: mark send transaction as expired
                        log::warn!("Transaction expired: {:#?}", msg_hash);
                    }
                    !expired
                });
            }
        });
    }
}

#[derive(Debug)]
pub struct TransactionContext {
    account: UInt256,
    transaction_hash: UInt256,
    transaction: ton_block::Transaction,
}

struct TransactionObserver {
    tx: mpsc::UnboundedSender<TransactionContext>,
}

impl TransactionsSubscription for TransactionObserver {
    fn handle_transaction(&self, ctx: TxContext<'_>) -> Result<()> {
        let transaction = TransactionContext {
            account: *ctx.account,
            transaction_hash: *ctx.transaction_hash,
            transaction: ctx.transaction.clone(),
        };

        self.tx.send(transaction)?;

        // Done
        Ok(())
    }
}

#[derive(Debug)]
pub struct TokenTransactionContext {
    account: UInt256,
    block_hash: UInt256,
    block_utime: u32,
    message_hash: UInt256,
    transaction_hash: UInt256,
    transaction: ton_block::Transaction,
    shard_accounts: ton_block::ShardAccounts,
}

struct TokenTransactionObserver {
    tx: mpsc::UnboundedSender<(TokenTransactionContext, TokenWalletTransaction)>,
}

impl TransactionsSubscription for TokenTransactionObserver {
    fn handle_transaction(&self, ctx: TxContext<'_>) -> Result<()> {
        if ctx.transaction_info.aborted {
            return Ok(());
        }

        let parsed = nekoton::core::parsing::parse_token_transaction(
            ctx.transaction,
            ctx.transaction_info,
            TokenWalletVersion::Tip3v4,
        );

        if let Some(parsed) = parsed {
            let message_hash = match &parsed {
                TokenWalletTransaction::IncomingTransfer(_)
                | TokenWalletTransaction::Accept(_)
                | TokenWalletTransaction::TransferBounced(_)
                | TokenWalletTransaction::SwapBackBounced(_) => ctx
                    .transaction
                    .in_msg
                    .clone()
                    .map(|message| message.hash())
                    .unwrap_or_default(),
                TokenWalletTransaction::OutgoingTransfer(_)
                | TokenWalletTransaction::SwapBack(_) => {
                    let mut hash = Default::default();
                    let _ = ctx.transaction.out_msgs.iterate(|message| {
                        hash = message.hash().unwrap_or_default();
                        Ok(false)
                    });
                    hash
                }
            };

            self.tx
                .send((
                    TokenTransactionContext {
                        account: *ctx.account,
                        block_hash: *ctx.block_hash,
                        block_utime: ctx.block_info.gen_utime().0,
                        message_hash,
                        transaction_hash: *ctx.transaction_hash,
                        transaction: ctx.transaction.clone(),
                        shard_accounts: ctx.shard_accounts.clone(),
                    },
                    parsed,
                ))
                .ok();
        }

        // Done
        Ok(())
    }
}

#[derive(Deserialize, Clone)]
pub struct TonCoreConfig {
    pub ton_indexer: ton_indexer::NodeConfig,
}

pub enum ReceiveTransaction {
    Create(CreateReceiveTransaction),
    UpdateSent(UpdateSentTransaction),
}

pub enum ReceiveTokenTransaction {
    Create(CreateReceiveTokenTransaction),
    UpdateSent(UpdateSentTokenTransaction),
}

pub type ReceiveTransactionTx = mpsc::UnboundedSender<ReceiveTransaction>;
pub type ReceiveTransactionRx = mpsc::UnboundedReceiver<ReceiveTransaction>;

pub type ReceiveTokenTransactionTx = mpsc::UnboundedSender<ReceiveTokenTransaction>;
pub type ReceiveTokenTransactionRx = mpsc::UnboundedReceiver<ReceiveTokenTransaction>;

#[derive(thiserror::Error, Debug)]
enum TonCoreError {
    #[error("Already initialized")]
    AlreadyInitialized,
    #[error("External ton message expected")]
    ExternalTonMessageExpected,
    #[error("Pending message hash `{0}` exist")]
    PendingMessageExist(String),
    #[error("Pending message hash `{0}` not exist")]
    PendingMessageNotExist(String),
    #[error("Account `{0}` not found")]
    AccountNotFound(String),
    #[error("Failed to handle transaction")]
    WrongTransaction,
}
