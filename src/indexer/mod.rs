use std::collections::{hash_map, HashMap};
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use parking_lot::Mutex;
use serde::Deserialize;
use tokio::sync::mpsc;
use ton_block::{GetRepresentationHash, Serializable};
use ton_types::UInt256;

use self::models::*;
use self::ton_contracts::*;
use self::ton_subscriber::*;
use dexpa::macros::failure::err_msg;
use hyper::service::make_service_fn;

mod models;
mod ton_contracts;
mod ton_subscriber;

pub struct TonIndexer {
    ton_engine: Arc<ton_indexer::Engine>,
    ton_subscriber: Arc<TonSubscriber>,

    account_observer: Arc<AccountObserver>,
    ext_in_msg_cache: Mutex<HashMap<UInt256, i64>>,

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

        let (account_transaction_tx, account_transaction_rx) = mpsc::unbounded_channel();

        let engine = Arc::new(Self {
            ton_engine,
            ton_subscriber,
            account_observer: Arc::new(AccountObserver {
                tx: account_transaction_tx,
            }),
            ext_in_msg_cache: Mutex::new(HashMap::new()),
            initialized: Default::default(),
        });

        engine.start_listening_accounts_transactions(account_transaction_rx);
        engine.start_ext_in_msg_cache_watcher();

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

    pub async fn get_ton_address_info(&self, account: UInt256) -> Result<TonAddressInfo> {
        let contract = self
            .ton_subscriber
            .get_contract_state(account)
            .await?
            .unwrap();

        let workchain_id = contract.account.addr.workchain_id();
        let hex = contract.account.addr.address().to_hex_string();
        let account_status = contract.account.storage.state;
        let network_balance = contract.account.storage.balance.grams.value();

        let mut last_transaction_hash = None;
        let mut last_transaction_lt = None;
        if let nekoton_abi::LastTransactionId::Exact(transaction_id) = contract.last_transaction_id
        {
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
            sync_u_time: 0,
        })
    }

    pub async fn get_token_address_info(&self, account: UInt256) -> Result<TokenAddressInfo> {
        let contract = self
            .ton_subscriber
            .get_contract_state(account)
            .await?
            .unwrap();

        let token_wallet = TonTokenWalletContract(&contract);
        let root_address = token_wallet.get_details()?.root_address;

        let workchain_id = contract.account.addr.workchain_id();
        let hex = contract.account.addr.address().to_hex_string();
        let account_status = contract.account.storage.state;
        let network_balance = contract.account.storage.balance.grams.value();

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
            sync_u_time: 0,
        })
    }

    pub async fn get_version(&self, account: UInt256) -> Result<u32> {
        let contract = self
            .ton_subscriber
            .get_contract_state(account)
            .await?
            .unwrap();

        let token_wallet = TonTokenWalletContract(&contract);
        token_wallet.get_version()
    }

    pub async fn send_ton_message(&self, message: &ton_block::Message) -> Result<()> {
        let to = match message.header() {
            ton_block::CommonMsgInfo::ExtInMsgInfo(header) => {
                self.add_ext_in_msg_to_cache(message)?;
                ton_block::AccountIdPrefixFull::prefix(&header.dst)?
            }
            _ => return Err(EngineError::ExternalTonMessageExpected.into()),
        };

        let cells = message.write_to_new_cell()?.into();
        let serialized = ton_types::serialize_toc(&cells)?;

        self.ton_engine
            .broadcast_external_message(&to, &serialized)
            .await
    }

    pub fn add_account_subscription<I>(&self, accounts: I)
    where
        I: IntoIterator<Item = UInt256>,
    {
        self.ton_subscriber
            .add_transactions_subscription(accounts, &self.account_observer);
    }

    fn start_listening_accounts_transactions(
        self: &Arc<Self>,
        mut rx: mpsc::UnboundedReceiver<AccountTransaction>,
    ) {
        let engine = Arc::downgrade(self);

        tokio::spawn(async move {
            while let Some(transaction) = rx.recv().await {
                let engine = match engine.upgrade() {
                    Some(engine) => engine,
                    None => break,
                };

                log::info!("Transaction: {:#?}", transaction);

                if let Some(in_msg) = transaction
                    .transaction
                    .in_msg
                    .as_ref()
                    .and_then(|data| data.read_struct().ok())
                {
                    if let ton_block::CommonMsgInfo::ExtInMsgInfo(header) = in_msg.header() {
                        if let Err(err) = engine.remove_ext_in_msg_from_cache(&in_msg) {
                            todo!()
                        }
                        log::info!("Message: {:#?}", in_msg);
                    }
                }

                // TODO: Create receive transaction
            }

            rx.close();
            while rx.recv().await.is_some() {}
        });
    }

    fn add_ext_in_msg_to_cache(&self, message: &ton_block::Message) -> Result<()> {
        let mut msg_cache = self.ext_in_msg_cache.lock();
        match msg_cache.entry(message.hash()?) {
            hash_map::Entry::Vacant(entry) => {
                entry.insert(Utc::now().timestamp() + 60);
            }
            hash_map::Entry::Occupied(_) => {
                return Err(EngineError::ExternalTonMessageExistInCache.into());
            }
        };
        Ok(())
    }

    fn remove_ext_in_msg_from_cache(&self, message: &ton_block::Message) -> Result<()> {
        let mut msg_cache = self.ext_in_msg_cache.lock();
        if let None = msg_cache.remove(&message.hash()?) {
            return Err(EngineError::ExternalTonMessageNotExistInCache.into());
        }
        Ok(())
    }

    fn start_ext_in_msg_cache_watcher(self: &Arc<Self>) {
        let engine = Arc::downgrade(self);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(1000));

            while let Some(engine) = engine.upgrade() {
                interval.tick().await;

                let now = Utc::now().timestamp();
                let mut msg_cache = engine.ext_in_msg_cache.lock();

                msg_cache.retain(|_, expired| {
                    let mut keep = true;
                    if now > *expired {
                        // TODO: Mark sent transaction as bad

                        keep = false;
                    }

                    keep
                });
            }
        });
    }
}

#[derive(Debug)]
struct AccountTransaction {
    account: UInt256,
    transaction_hash: UInt256,
    transaction: ton_block::Transaction,
}

struct AccountObserver {
    tx: mpsc::UnboundedSender<AccountTransaction>,
}

impl TransactionsSubscription for AccountObserver {
    fn handle_transaction(&self, ctx: TxContext<'_>) -> Result<()> {
        let transaction = AccountTransaction {
            account: *ctx.account,
            transaction_hash: *ctx.transaction_hash,
            transaction: ctx.transaction.clone(),
        };

        self.tx.send(transaction)?;

        // Done
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
    #[error("External ton message expected")]
    ExternalTonMessageExpected,
    #[error("External ton message to send is already in cache")]
    ExternalTonMessageExistInCache,
    #[error("Received external ton message is not in cache")]
    ExternalTonMessageNotExistInCache,
}
