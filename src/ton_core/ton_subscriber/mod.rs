use std::collections::hash_map;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Weak};

use anyhow::Result;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use nekoton::core::models::TokenWalletVersion;
use nekoton::transport::models::ExistingContract;
use nekoton_utils::TrustMe;
use parking_lot::{Mutex, RwLock, RwLockReadGuard};
use rustc_hash::FxHashMap;

use tokio::sync::Notify;
use ton_block::{Deserializable, HashmapAugType, ShardIdent};
use ton_indexer::utils::{BlockIdExtExtension, RefMcStateHandle, ShardStateStuff};
use ton_indexer::{BriefBlockMeta, EngineStatus, ProcessBlockContext};
use ton_types::{HashmapType, UInt256};

use crate::ton_core::*;

pub struct TonSubscriber {
    ready: AtomicBool,
    ready_signal: Notify,
    current_utime: AtomicU32,
    signature_id: SignatureId,
    state_subscriptions: RwLock<FxHashMap<UInt256, StateSubscription>>,
    token_subscription: RwLock<Option<TokenSubscription>>,
    full_state_subscription: RwLock<Option<FullStateSubscription>>,
    shards_accounts_cache: RwLock<FxHashMap<ShardIdent, ShardAccounts>>,
    mc_block_awaiters: Mutex<FxHashMap<usize, Box<dyn BlockAwaiter>>>,
    messages_queue: Arc<PendingMessagesQueue>,
}

impl TonSubscriber {
    pub fn new(messages_queue: Arc<PendingMessagesQueue>) -> Arc<Self> {
        Arc::new(Self {
            ready: AtomicBool::new(false),
            ready_signal: Notify::new(),
            current_utime: AtomicU32::new(0),
            signature_id: SignatureId::default(),
            state_subscriptions: RwLock::new(FxHashMap::with_capacity_and_hasher(
                1024,
                Default::default(),
            )),
            token_subscription: Default::default(),
            full_state_subscription: Default::default(),
            shards_accounts_cache: RwLock::new(FxHashMap::with_capacity_and_hasher(
                16,
                Default::default(),
            )),
            mc_block_awaiters: Mutex::new(FxHashMap::with_capacity_and_hasher(
                4,
                Default::default(),
            )),
            messages_queue,
        })
    }

    pub fn metrics(&self) -> TonSubscriberMetrics {
        TonSubscriberMetrics {
            ready: self.ready.load(Ordering::Acquire),
            current_utime: self.current_utime(),
            signature_id: self.signature_id(),
            pending_message_count: self.messages_queue.len(),
        }
    }

    pub async fn start(self: &Arc<Self>, engine: &ton_indexer::Engine) -> Result<()> {
        let last_key_block = engine.load_last_key_block().await?;
        self.update_signature_id(last_key_block.block())?;

        self.wait_sync().await;
        Ok(())
    }

    pub fn current_utime(&self) -> u32 {
        self.current_utime.load(Ordering::Acquire)
    }

    pub fn signature_id(&self) -> Option<i32> {
        self.signature_id.load()
    }

    pub fn add_transactions_subscription<I, T>(&self, accounts: I, subscription: &Arc<T>)
    where
        I: IntoIterator<Item = UInt256>,
        T: TransactionsSubscription + 'static,
    {
        let mut state_subscriptions = self.state_subscriptions.write();

        let weak = Arc::downgrade(subscription) as Weak<dyn TransactionsSubscription>;

        for account in accounts {
            match state_subscriptions.entry(account) {
                hash_map::Entry::Vacant(entry) => {
                    entry.insert(StateSubscription {
                        transaction_subscriptions: vec![weak.clone()],
                    });
                }
                hash_map::Entry::Occupied(mut entry) => {
                    entry.get_mut().transaction_subscriptions.push(weak.clone());
                }
            };
        }
    }

    pub fn add_token_subscription<T>(&self, subscription: &Arc<T>)
    where
        T: TransactionsSubscription + 'static,
    {
        let mut token_subscription = self.token_subscription.write();

        let weak = Arc::downgrade(subscription) as Weak<dyn TransactionsSubscription>;

        let _ = token_subscription.insert(TokenSubscription {
            transaction_subscription: weak.clone(),
        });
    }

    pub fn add_full_state_subscription<T>(&self, subscription: &Arc<T>)
    where
        T: FullStatesSubscription + 'static,
    {
        let mut full_state_subscription = self.full_state_subscription.write();

        let weak = Arc::downgrade(subscription) as Weak<dyn FullStatesSubscription>;

        let _ = full_state_subscription.insert(FullStateSubscription {
            full_state_subscription: weak.clone(),
        });
    }

    pub fn get_contract_state(&self, account: &UInt256) -> Result<Option<ShardAccount>> {
        let cache = self.shards_accounts_cache.read();
        for (shard_ident, shard_accounts) in cache.iter() {
            if !contains_account(shard_ident, account) {
                continue;
            }
            return shard_accounts.get(account);
        }
        Ok(None)
    }

    pub fn update_shards_accounts_cache(
        &self,
        shard_id: ShardIdent,
        shard_state: Arc<ShardStateStuff>,
    ) -> Result<()> {
        let shard_accounts = shard_state.state().read_accounts()?;
        let state_handle = shard_state.ref_mc_state_handle().clone();

        let mut shards_accounts = self.shards_accounts_cache.write();
        shards_accounts.insert(
            shard_id,
            ShardAccounts {
                accounts: shard_accounts,
                state_handle,
            },
        );

        Ok(())
    }

    fn handle_masterchain_block(
        &self,
        meta: BriefBlockMeta,
        block: &ton_block::Block,
    ) -> Result<()> {
        let gen_utime = meta.gen_utime();
        self.current_utime.store(gen_utime, Ordering::Release);

        let block_info = block.info.read_struct()?;
        if block_info.key_block() {
            self.update_signature_id(block)?;
        }

        if !self.ready.load(Ordering::Acquire) {
            return Ok(());
        }

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
        shard_state: Option<&ShardStateStuff>,
        block_hash: &UInt256,
    ) -> Result<FuturesUnordered<HandleTransactionStatusRx>> {
        let block_info = block.info.read_struct()?;
        let extra = block.extra.read_struct()?;
        let account_blocks = extra.read_account_blocks()?;

        if let Some(shard_state) = shard_state {
            let shard_accounts = shard_state.state().read_accounts()?;
            let state_handle = shard_state.ref_mc_state_handle().clone();

            let mut shards_accounts = self.shards_accounts_cache.write();
            shards_accounts.insert(
                *block_info.shard(),
                ShardAccounts {
                    accounts: shard_accounts,
                    state_handle,
                },
            );
            if block_info.after_merge() || block_info.after_split() {
                let block_ids = block_info.read_prev_ids()?;
                match block_ids.len() {
                    1 => {
                        let (left, right) = block_ids[0].shard_id.split()?;
                        if shards_accounts.contains_key(&left)
                            && shards_accounts.contains_key(&right)
                        {
                            shards_accounts.remove(&block_ids[0].shard_id);
                        }
                    }
                    len if len > 1 => {
                        for block_id in block_info.read_prev_ids()? {
                            shards_accounts.remove(&block_id.shard_id);
                        }
                    }
                    _ => {}
                }
            }
            drop(shards_accounts);
        }

        let mut states = FuturesUnordered::new();

        let state_subscriptions = self.state_subscriptions.read();
        let token_subscription = self.token_subscription.read();
        let shards_accounts_cache = self.shards_accounts_cache.read();

        account_blocks.iterate_with_keys(|account, account_block| {
            match state_subscriptions.get(&account) {
                Some(subscription) => {
                    match subscription.handle_block(
                        &self.messages_queue,
                        &block_info,
                        &account_block,
                        &account,
                        block_hash,
                    ) {
                        Ok(rx_states) => {
                            if !rx_states.is_empty() {
                                states.extend(rx_states);
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to handle block: {:?}", e);
                        }
                    };
                }
                None => {
                    let token_subscription = token_subscription.as_ref().trust_me();

                    match token_subscription.handle_block(
                        &state_subscriptions,
                        &shards_accounts_cache,
                        &block_info,
                        &account_block,
                        &account,
                        block_hash,
                    ) {
                        Ok(rx_states) => {
                            if !rx_states.is_empty() {
                                states.extend(rx_states);
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to handle block: {:?}", e);
                        }
                    }
                }
            };

            Ok(true)
        })?;

        self.messages_queue
            .update(block_info.shard(), block_info.gen_utime().as_u32());

        Ok(states)
    }

    fn handle_full_state(
        &self,
        state: &ShardStateStuff,
    ) -> Result<Option<HandleTransactionStatusRx>> {
        let shard_accounts = state.state().read_accounts()?;
        let state_handle = state.ref_mc_state_handle().clone();

        let mut shards_accounts = self.shards_accounts_cache.write();
        shards_accounts.insert(
            *state.shard(),
            ShardAccounts {
                accounts: shard_accounts,
                state_handle,
            },
        );
        drop(shards_accounts);

        let full_state_subscription = self.full_state_subscription.read();
        full_state_subscription
            .as_ref()
            .trust_me()
            .handle_full_state(state)
    }

    fn update_signature_id(&self, key_block: &ton_block::Block) -> Result<()> {
        let extra = key_block.read_extra()?;
        let custom = extra
            .read_custom()?
            .context("McBlockExtra not found in the masterchain block")?;
        let config = custom
            .config()
            .context("Config not found in the key block")?;

        self.signature_id
            .store(config.capabilities(), key_block.global_id);

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

    async fn process_block(&self, ctx: ProcessBlockContext<'_>) -> Result<()> {
        if ctx.block_stuff().id().is_masterchain() {
            self.handle_masterchain_block(ctx.meta(), ctx.block())?;
        } else {
            let mut states = self.handle_shard_block(
                ctx.block(),
                ctx.shard_state_stuff(),
                &ctx.block_stuff().id().root_hash,
            )?;
            while let Some(status) = states.next().await {
                if let Err(err) = status {
                    log::error!("Failed to receive transaction status: {}", err);
                }
            }
        }

        Ok(())
    }

    async fn process_full_state(&self, state: &ShardStateStuff) -> Result<()> {
        if state.block_id().shard_id.is_masterchain() {
            return Ok(());
        }

        let res = self.handle_full_state(state)?;
        if let Some(rx) = res {
            rx.await?;
        }

        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct TonSubscriberMetrics {
    pub ready: bool,
    pub current_utime: u32,
    pub signature_id: Option<i32>,
    pub pending_message_count: usize,
}

struct StateSubscription {
    transaction_subscriptions: Vec<Weak<dyn TransactionsSubscription>>,
}

impl StateSubscription {
    fn handle_block(
        &self,
        messages_queue: &PendingMessagesQueue,
        block_info: &ton_block::BlockInfo,
        account_block: &ton_block::AccountBlock,
        account: &UInt256,
        block_hash: &UInt256,
    ) -> Result<FuturesUnordered<HandleTransactionStatusRx>> {
        let states = FuturesUnordered::new();

        if self.transaction_subscriptions.is_empty() {
            return Ok(states);
        }

        for transaction in account_block.transactions().iter() {
            let result = transaction.and_then(|(_, value)| {
                let cell = value.into_cell().reference(0)?;
                let hash = cell.repr_hash();

                ton_block::Transaction::construct_from_cell(cell)
                    .map(|transaction| (hash, transaction))
            });

            let (hash, transaction) = match result {
                Ok(result) => result,
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
                block_info,
                block_hash,
                account,
                transaction_hash: &hash,
                transaction_info: &transaction_info,
                transaction: &transaction,
                in_msg: &in_msg,
                token_transaction: &None,
                token_state: &None,
            };

            // Handle transaction
            for subscription in self.iter_transaction_subscriptions() {
                let (tx, rx) = oneshot::channel();
                match subscription.handle_transaction(ctx, tx) {
                    Ok(_) => {
                        states.push(rx);
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to handle transaction {} for account {}: {:?}",
                            hash.to_hex_string(),
                            account.to_hex_string(),
                            e
                        );
                    }
                };
            }
        }

        Ok(states)
    }

    fn iter_transaction_subscriptions(
        &'_ self,
    ) -> impl Iterator<Item = Arc<dyn TransactionsSubscription>> + '_ {
        self.transaction_subscriptions
            .iter()
            .filter_map(Weak::upgrade)
    }
}

struct TokenSubscription {
    transaction_subscription: Weak<dyn TransactionsSubscription>,
}

impl TokenSubscription {
    fn handle_block(
        &self,
        state_subscriptions: &RwLockReadGuard<FxHashMap<UInt256, StateSubscription>>,
        shards_accounts_cache: &FxHashMap<ShardIdent, ShardAccounts>,
        block_info: &ton_block::BlockInfo,
        account_block: &ton_block::AccountBlock,
        account: &UInt256,
        block_hash: &UInt256,
    ) -> Result<FuturesUnordered<HandleTransactionStatusRx>> {
        let states = FuturesUnordered::new();

        for transaction in account_block.transactions().iter() {
            let result = transaction.and_then(|(_, value)| {
                let cell = value.into_cell().reference(0)?;
                let hash = cell.repr_hash();

                ton_block::Transaction::construct_from_cell(cell)
                    .map(|transaction| (hash, transaction))
            });

            let (hash, transaction) = match result {
                Ok(result) => result,
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

            let parsed_token_transaction = match nekoton::core::parsing::parse_token_transaction(
                &transaction,
                &transaction_info,
                TokenWalletVersion::Tip3,
            ) {
                Some(parsed_token_transaction) => Some(parsed_token_transaction),
                None => nekoton::core::parsing::parse_token_transaction(
                    &transaction,
                    &transaction_info,
                    TokenWalletVersion::OldTip3v4,
                ),
            };

            if let Some(parsed) = parsed_token_transaction {
                let token_contract = shards_accounts_cache
                    .find_account(account)?
                    .ok_or_else(|| TonCoreError::AccountNotExist(account.to_string()))?;

                let (token_wallet_details, ..) = get_token_wallet_details(&token_contract)?;
                let owner_account = UInt256::from_be_bytes(
                    &token_wallet_details
                        .owner_address
                        .address()
                        .get_bytestring(0),
                );

                if state_subscriptions.get(&owner_account).is_some() {
                    let in_msg = match transaction
                        .in_msg
                        .as_ref()
                        .map(|message| (message, message.read_struct()))
                    {
                        Some((_, Ok(message))) => message,
                        _ => continue,
                    };

                    let ctx = TxContext {
                        block_info,
                        block_hash,
                        account,
                        transaction_hash: &hash,
                        transaction_info: &transaction_info,
                        transaction: &transaction,
                        in_msg: &in_msg,
                        token_transaction: &Some(parsed),
                        token_state: &Some(token_contract),
                    };

                    if let Some(transaction_subscription) = self.transaction_subscription.upgrade()
                    {
                        let (tx, rx) = oneshot::channel();

                        match transaction_subscription.handle_transaction(ctx, tx) {
                            Ok(_) => {
                                states.push(rx);
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to handle token transaction {} for account {}: {:?}",
                                    hash.to_hex_string(),
                                    account.to_hex_string(),
                                    e
                                );
                            }
                        };
                    }
                }
            }
        }

        Ok(states)
    }
}

struct FullStateSubscription {
    full_state_subscription: Weak<dyn FullStatesSubscription>,
}

impl FullStateSubscription {
    fn handle_full_state(
        &self,
        shard_state: &ShardStateStuff,
    ) -> Result<Option<HandleTransactionStatusRx>> {
        let mut res = None;

        if let Some(full_state_subscription) = self.full_state_subscription.upgrade() {
            let (tx, rx) = oneshot::channel();

            let ctx = StateContext {
                block_id: shard_state.block_id(),
            };

            match full_state_subscription.handle_full_state(ctx, tx) {
                Ok(_) => res = Some(rx),
                Err(e) => {
                    log::error!("Failed to handle full state: {:?}", e);
                }
            };
        }

        Ok(res)
    }
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
    fn handle_transaction(
        &self,
        ctx: TxContext<'_>,
        state: HandleTransactionStatusTx,
    ) -> Result<()>;
}

pub trait FullStatesSubscription: Send + Sync {
    fn handle_full_state(
        &self,
        ctx: StateContext<'_>,
        state: HandleTransactionStatusTx,
    ) -> Result<()>;
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
    fn handle_transaction(
        &self,
        ctx: TxContext<'_>,
        state: HandleTransactionStatusTx,
    ) -> Result<()> {
        let event = T::read_from_transaction(&ctx, state);

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

impl<T> FullStatesSubscription for AccountObserver<T>
where
    T: ReadFromState + Send + Sync,
{
    fn handle_full_state(&self, ctx: StateContext, state: HandleTransactionStatusTx) -> Result<()> {
        let event = T::read_from_state(&ctx, state);

        // Send event to event manager if it exist
        if self.0.send(event).is_err() {
            log::error!("Failed to send event: channel is dropped");
        }

        // Done
        Ok(())
    }
}

pub struct ShardAccount {
    data: ton_types::Cell,
    last_transaction_id: LastTransactionId,
    _state_handle: Arc<RefMcStateHandle>,
}

pub fn make_existing_contract(state: Option<ShardAccount>) -> Result<Option<ExistingContract>> {
    let state = match state {
        Some(this) => this,
        None => return Ok(None),
    };

    match ton_block::Account::construct_from_cell(state.data)? {
        ton_block::Account::AccountNone => Ok(None),
        ton_block::Account::Account(account) => Ok(Some(ExistingContract {
            account,
            timings: GenTimings::Unknown,
            last_transaction_id: state.last_transaction_id,
        })),
    }
}

pub struct ShardAccounts {
    pub accounts: ton_block::ShardAccounts,
    pub state_handle: Arc<RefMcStateHandle>,
}

impl ShardAccounts {
    fn get(&self, account: &UInt256) -> Result<Option<ShardAccount>> {
        match self.accounts.get(account)? {
            Some(account) => Ok(Some(ShardAccount {
                data: account.account_cell(),
                last_transaction_id: LastTransactionId::Exact(TransactionId {
                    lt: account.last_trans_lt(),
                    hash: *account.last_trans_hash(),
                }),
                _state_handle: self.state_handle.clone(),
            })),
            None => Ok(None),
        }
    }
}

impl ShardAccountsMapExt for FxHashMap<ShardIdent, ShardAccounts> {
    fn find_account(&self, account: &UInt256) -> Result<Option<ExistingContract>> {
        let item = self
            .iter()
            .find(|(shard_ident, _)| contains_account(shard_ident, account));

        match item {
            Some((_, shard)) => shard.accounts.find_account(account),
            None => Err(TonCoreError::InvalidContractAddress).context("No suitable shard found"),
        }
    }
}

#[derive(Default)]
struct SignatureId(AtomicU64);

impl SignatureId {
    const WITH_SIGNATURE_ID: u64 = 1 << 32;

    fn load(&self) -> Option<i32> {
        let id = self.0.load(Ordering::Acquire);
        if id & Self::WITH_SIGNATURE_ID != 0 {
            Some(id as i32)
        } else {
            None
        }
    }

    fn store(&self, capabilities: u64, global_id: i32) {
        const CAP_WITH_SIGNATURE_ID: u64 = 0x4000000;
        let id = if capabilities & CAP_WITH_SIGNATURE_ID != 0 {
            Self::WITH_SIGNATURE_ID | (global_id as u32 as u64)
        } else {
            0
        };
        self.0.store(id, Ordering::Release);
    }
}

pub type AccountEventsTx<T> = mpsc::UnboundedSender<T>;
