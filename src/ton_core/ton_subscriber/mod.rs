use std::collections::{hash_map, HashMap};
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Weak};

use anyhow::{Context, Result};
use nekoton::transport::models::ExistingContract;
use parking_lot::Mutex;
use tiny_adnl::utils::FxHashMap;
use tokio::sync::{watch, Notify};
use ton_block::{Deserializable, HashmapAugType};
use ton_indexer::utils::{BlockIdExtExtension, BlockProofStuff, BlockStuff, ShardStateStuff};
use ton_indexer::EngineStatus;
use ton_types::{HashmapType, UInt256};

use crate::ton_core::*;

pub struct TonSubscriber {
    ready: AtomicBool,
    ready_signal: Notify,
    current_utime: AtomicU32,
    state_subscriptions: Mutex<HashMap<UInt256, StateSubscription>>,
    shard_accounts_cache: Mutex<HashMap<ton_block::ShardIdent, ton_block::ShardAccounts>>,
    mc_block_awaiters: Mutex<FxHashMap<usize, Box<dyn BlockAwaiter>>>,
    messages_queue: Arc<PendingMessagesQueue>,
}

impl TonSubscriber {
    pub fn new(messages_queue: Arc<PendingMessagesQueue>) -> Arc<Self> {
        Arc::new(Self {
            ready: AtomicBool::new(false),
            ready_signal: Notify::new(),
            current_utime: AtomicU32::new(0),
            state_subscriptions: Mutex::new(HashMap::new()),
            shard_accounts_cache: Mutex::new(HashMap::new()),
            mc_block_awaiters: Mutex::new(FxHashMap::with_capacity_and_hasher(
                4,
                Default::default(),
            )),
            messages_queue,
        })
    }

    pub async fn start(self: &Arc<Self>) -> Result<()> {
        self.wait_sync().await;
        Ok(())
    }

    pub fn current_utime(&self) -> u32 {
        self.current_utime.load(Ordering::Acquire)
    }

    pub fn add_transactions_subscription<I, T>(&self, accounts: I, subscription: &Arc<T>)
    where
        I: IntoIterator<Item = UInt256>,
        T: TransactionsSubscription + 'static,
    {
        let mut state_subscriptions = self.state_subscriptions.lock();

        let weak = Arc::downgrade(subscription) as Weak<dyn TransactionsSubscription>;

        for account in accounts {
            match state_subscriptions.entry(account) {
                hash_map::Entry::Vacant(entry) => {
                    let (state_tx, state_rx) = watch::channel(None);
                    entry.insert(StateSubscription {
                        state_tx,
                        state_rx,
                        transaction_subscriptions: vec![weak.clone()],
                    });
                }
                hash_map::Entry::Occupied(mut entry) => {
                    entry.get_mut().transaction_subscriptions.push(weak.clone());
                }
            };
        }
    }

    #[allow(dead_code)]
    pub async fn wait_contract_state(&self, account: UInt256) -> Result<Option<ExistingContract>> {
        let mut state_rx = match self.state_subscriptions.lock().entry(account) {
            hash_map::Entry::Vacant(entry) => {
                let (state_tx, state_rx) = watch::channel(None);
                entry
                    .insert(StateSubscription {
                        state_tx,
                        state_rx,
                        transaction_subscriptions: Vec::new(),
                    })
                    .state_rx
                    .clone()
            }
            hash_map::Entry::Occupied(entry) => entry.get().state_rx.clone(),
        };

        state_rx.changed().await?;
        let account = state_rx.borrow_and_update();
        ExistingContract::from_shard_account_opt(account.deref())
    }

    pub fn get_contract_state(&self, account: &UInt256) -> Result<Option<ExistingContract>> {
        let state = None;

        let shard_accounts_cache = self.shard_accounts_cache.lock();
        for (shard_ident, shard_accounts) in shard_accounts_cache.iter() {
            if contains_account(shard_ident, account) {
                match shard_accounts.get(account) {
                    Ok(account) => return ExistingContract::from_shard_account_opt(&account),
                    Err(e) => {
                        log::error!("Failed to get account {}: {:?}", account.to_hex_string(), e);
                    }
                };
            }
        }

        Ok(state)
    }

    fn handle_masterchain_block(&self, block: &ton_block::Block) -> Result<()> {
        let block_info = block.info.read_struct()?;
        self.current_utime
            .store(block_info.gen_utime().0, Ordering::Release);

        let mut mc_block_awaiters = self.mc_block_awaiters.lock();
        mc_block_awaiters.retain(
            |_, awaiter| match awaiter.handle_block(block, &block_info) {
                Ok(action) => action == BlockAwaiterAction::Retain,
                Err(e) => {
                    log::error!("Failed to handle masterchain block: {:?}", e);
                    true
                }
            },
        );

        Ok(())
    }

    fn handle_shard_block(
        &self,
        block: &ton_block::Block,
        shard_state: &ton_block::ShardStateUnsplit,
        block_hash: &UInt256,
    ) -> Result<()> {
        let block_info = block.info.read_struct()?;
        let extra = block.extra.read_struct()?;
        let account_blocks = extra.read_account_blocks()?;
        let shard_accounts = shard_state.read_accounts()?;

        {
            let mut shard_accounts_cache = self.shard_accounts_cache.lock();
            shard_accounts_cache.insert(*block_info.shard(), shard_accounts.clone());
            if block_info.after_merge() || block_info.after_split() {
                let block_ids = block_info.read_prev_ids()?;
                match block_ids.len() {
                    1 => {
                        let (left, right) = block_ids[0].shard_id.split()?;
                        if shard_accounts_cache.contains_key(&left)
                            && shard_accounts_cache.contains_key(&right)
                        {
                            shard_accounts_cache.remove(&block_ids[0].shard_id);
                        }
                    }
                    len if len > 1 => {
                        for block_id in block_info.read_prev_ids()? {
                            shard_accounts_cache.remove(&block_id.shard_id);
                        }
                    }
                    _ => {}
                }
            }
        }

        {
            let mut subscriptions = self.state_subscriptions.lock();
            subscriptions.retain(|account, subscription| {
                let subscription_status = subscription.update_status();
                if subscription_status == StateSubscriptionStatus::Stopped {
                    return false;
                }

                if !contains_account(block_info.shard(), account) {
                    return true;
                }

                let mut keep = true;

                if subscription_status == StateSubscriptionStatus::Alive {
                    match shard_accounts.get(account) {
                        Ok(account) => {
                            if subscription.state_tx.send(account).is_err() {
                                log::error!("Shard subscription somehow dropped");
                                keep = false;
                            }
                        }
                        Err(e) => {
                            log::error!(
                                "Failed to get account {}: {:?}",
                                account.to_hex_string(),
                                e
                            );
                        }
                    };
                } else {
                    subscription.state_rx.borrow_and_update();
                }

                if let Err(e) = subscription.handle_block(
                    &self.messages_queue,
                    &shard_accounts,
                    &block_info,
                    &account_blocks,
                    account,
                    block_hash,
                ) {
                    log::error!("Failed to handle block: {:?}", e);
                }

                keep
            });
        }

        self.messages_queue
            .update(block_info.shard(), block_info.gen_utime().0);

        Ok(())
    }

    async fn wait_sync(&self) {
        if self.ready.load(Ordering::Acquire) {
            return;
        }
        self.ready_signal.notified().await;
    }
}

#[async_trait::async_trait]
impl ton_indexer::Subscriber for TonSubscriber {
    async fn engine_status_changed(&self, status: EngineStatus) {
        if status == EngineStatus::Synced {
            log::info!("TON subscriber is ready");
            self.ready.store(true, Ordering::Release);
            self.ready_signal.notify_waiters();
        }
    }

    async fn process_block(
        &self,
        block: &BlockStuff,
        _block_proof: Option<&BlockProofStuff>,
        shard_state: &ShardStateStuff,
    ) -> Result<()> {
        if block.id().is_masterchain() {
            self.handle_masterchain_block(block.block())?;
        } else {
            self.handle_shard_block(block.block(), shard_state.state(), &block.id().root_hash)?;
        }

        Ok(())
    }
}

struct StateSubscription {
    state_tx: ShardAccountTx,
    state_rx: ShardAccountRx,
    transaction_subscriptions: Vec<Weak<dyn TransactionsSubscription>>,
}

impl StateSubscription {
    fn update_status(&mut self) -> StateSubscriptionStatus {
        self.transaction_subscriptions
            .retain(|item| item.strong_count() > 0);

        if self.state_tx.receiver_count() > 1 {
            StateSubscriptionStatus::Alive
        } else if !self.transaction_subscriptions.is_empty() {
            StateSubscriptionStatus::PartlyAlive
        } else {
            StateSubscriptionStatus::Stopped
        }
    }

    fn handle_block(
        &self,
        messages_queue: &PendingMessagesQueue,
        shard_accounts: &ton_block::ShardAccounts,
        block_info: &ton_block::BlockInfo,
        account_blocks: &ton_block::ShardAccountBlocks,
        account: &UInt256,
        block_hash: &UInt256,
    ) -> Result<()> {
        if self.transaction_subscriptions.is_empty() {
            return Ok(());
        }

        let account_block = match account_blocks.get_with_aug(account).with_context(|| {
            format!(
                "Failed to get account block for {}",
                account.to_hex_string()
            )
        })? {
            Some((account_block, _)) => account_block,
            None => return Ok(()),
        };

        for transaction in account_block.transactions().iter() {
            let (hash, transaction) = match transaction.and_then(|(_, value)| {
                let cell = value.into_cell().reference(0)?;
                let hash = cell.repr_hash();

                ton_block::Transaction::construct_from_cell(cell)
                    .map(|transaction| (hash, transaction))
            }) {
                Ok(tx) => tx,
                Err(e) => {
                    log::error!(
                        "Failed to parse transaction in block {} for account {}: {:?}",
                        block_info.seq_no(),
                        account.to_hex_string(),
                        e
                    );
                    continue;
                }
            };

            // Skip non-ordinary transactions
            let transaction_info = match transaction.description.read_struct() {
                Ok(ton_block::TransactionDescr::Ordinary(info)) => info,
                _ => continue,
            };

            let in_msg = match transaction
                .in_msg
                .as_ref()
                .map(|message| (message, message.read_struct()))
            {
                Some((message_cell, Ok(message))) => {
                    if matches!(message.header(), ton_block::CommonMsgInfo::ExtInMsgInfo(_)) {
                        messages_queue.deliver_message(*account, message_cell.hash());
                    }
                    message
                }
                _ => continue,
            };

            let ctx = TxContext {
                shard_accounts,
                block_info,
                block_hash,
                account,
                transaction_hash: &hash,
                transaction_info: &transaction_info,
                transaction: &transaction,
                in_msg: &in_msg,
            };

            // Handle transaction
            for subscription in self.iter_transaction_subscriptions() {
                if let Err(e) = subscription.handle_transaction(ctx) {
                    log::error!(
                        "Failed to handle transaction {} for account {}: {:?}",
                        hash.to_hex_string(),
                        account.to_hex_string(),
                        e
                    );
                }
            }
        }

        Ok(())
    }

    fn iter_transaction_subscriptions(
        &'_ self,
    ) -> impl Iterator<Item = Arc<dyn TransactionsSubscription>> + '_ {
        self.transaction_subscriptions
            .iter()
            .map(Weak::upgrade)
            .flatten()
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum StateSubscriptionStatus {
    Alive,
    PartlyAlive,
    Stopped,
}

trait BlockAwaiter: Send + Sync {
    fn handle_block(
        &mut self,
        block: &ton_block::Block,
        block_info: &ton_block::BlockInfo,
    ) -> Result<BlockAwaiterAction>;
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum BlockAwaiterAction {
    Retain,
}

pub trait TransactionsSubscription: Send + Sync {
    fn handle_transaction(&self, ctx: TxContext<'_>) -> Result<()>;
}

/// Generic listener for transactions
pub struct AccountObserver<T>(AccountEventsTx<T>);

impl<T> AccountObserver<T> {
    pub fn new(tx: AccountEventsTx<T>) -> Arc<Self> {
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
            if self.0.send(event).is_err() {
                log::error!("Failed to send event: channel is dropped");
            }
        }

        // Done
        Ok(())
    }
}

type ShardAccountTx = watch::Sender<Option<ton_block::ShardAccount>>;
type ShardAccountRx = watch::Receiver<Option<ton_block::ShardAccount>>;

pub type AccountEventsTx<T> = mpsc::UnboundedSender<T>;
