use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use nekoton::core::models::{TokenWalletTransaction, TokenWalletVersion};
use nekoton::transport::models::{ExistingContract, RawContractState};
use serde::Deserialize;
use tokio::sync::mpsc;
use ton_block::{GetRepresentationHash, MsgAddressInt, Serializable};
use ton_types::UInt256;

use self::settings::*;
use self::ton_subscriber::*;
use self::transaction_parser::*;
use crate::models::*;
use crate::utils::*;

mod settings;
mod ton_subscriber;
mod transaction_parser;

pub struct TonCore {
    ton_engine: Arc<ton_indexer::Engine>,
    ton_subscriber: Arc<TonSubscriber>,

    owners_cache: OwnersCache,
    messages_queue: Arc<PendingMessagesQueue>,

    transaction_observer: Arc<TransactionObserver>,
    transaction_producer: ReceiveTransactionTx,

    token_transaction_observer: Arc<TokenTransactionObserver>,
    token_transaction_producer: ReceiveTokenTransactionTx,

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
        let node_config = get_node_config(&config).await?;

        let messages_queue = PendingMessagesQueue::new(100);
        let ton_subscriber = TonSubscriber::new(messages_queue.clone());

        let ton_engine = ton_indexer::Engine::new(
            node_config,
            global_config,
            vec![ton_subscriber.clone() as Arc<dyn ton_indexer::Subscriber>],
        )
        .await?;

        let (transaction_tx, transaction_rx) = mpsc::unbounded_channel();
        let (token_transaction_tx, token_transaction_rx) = mpsc::unbounded_channel();

        let engine = Arc::new(Self {
            ton_engine,
            owners_cache,
            messages_queue,
            ton_subscriber,
            transaction_producer,
            transaction_observer: Arc::new(TransactionObserver { tx: transaction_tx }),
            token_transaction_producer,
            token_transaction_observer: Arc::new(TokenTransactionObserver {
                tx: token_transaction_tx,
            }),
            initialized: Default::default(),
        });

        engine.start_listening_transactions(transaction_rx);
        engine.start_listening_token_transactions(token_transaction_rx);

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

    pub async fn get_contract_state(&self, account: UInt256) -> Result<ExistingContract> {
        match self.ton_subscriber.get_contract_state(account).await? {
            RawContractState::Exists(contract) => Ok(contract),
            RawContractState::NotExists => {
                Err(TonCoreError::AccountNotFound(account.to_hex_string()).into())
            }
        }
    }

    pub async fn send_ton_message(
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

    pub fn get_current_utime(&self) -> u32 {
        self.ton_subscriber.get_current_utime()
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

                match handle_transaction(transaction_ctx, &engine.owners_cache).await {
                    Ok(transaction) => {
                        engine.transaction_producer.send(transaction).ok();
                    }
                    Err(e) => {
                        log::error!("Failed to handle received transaction: {}", e);
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

        Ok(())
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct TonCoreConfig {
    pub port: u16,
    pub rocks_db_path: PathBuf,
    pub file_db_path: PathBuf,
    pub keys_path: PathBuf,
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
    #[error("Account `{0}` not found")]
    AccountNotFound(String),
}
