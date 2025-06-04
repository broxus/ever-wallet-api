use std::collections::hash_map;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Weak};

use anyhow::Result;
use everscale_types::boc::Boc;
use everscale_types::cell::{Cell, HashBytes, Load};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use nekoton::core::models::TokenWalletVersion;
use nekoton::transport::models::ExistingContract;
use nekoton_utils::TrustMe;
use parking_lot::{Mutex, RwLock, RwLockReadGuard};
use rustc_hash::FxHashMap;

use tokio::sync::Notify;
use ton_block::ExtraCurrencyCollection;
use tycho_block_util::block::BlockStuff;
use tycho_block_util::state::{RefMcStateHandle, ShardStateStuff};
use tycho_core::block_strider::StateSubscriberContext;
use tycho_vm::StackValue;

use crate::ton_core::*;

pub struct TonSubscriber {
    ready: AtomicBool,
    ready_signal: Notify,
    // tip block timestamp
    current_utime: AtomicU32,
    signature_id: SignatureId,
    state_subscriptions: RwLock<FxHashMap<HashBytes, StateSubscription>>,
    token_subscription: RwLock<Option<TokenSubscription>>,
    full_state_subscription: RwLock<Option<FullStateSubscription>>,
    mc_accounts: RwLock<Option<CachedAccounts>>,
    sc_accounts: RwLock<FxHashMap<ShardIdent, CachedAccounts>>,
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
            mc_accounts: Default::default(),
            sc_accounts: RwLock::new(FxHashMap::with_capacity_and_hasher(16, Default::default())),
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

    pub async fn start(self: &Arc<Self>, last_key_block: Option<BlockStuff>) -> Result<()> {
        if let Some(last_key_block) = last_key_block {
            self.update_signature_id(last_key_block.block())?;
        }

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
        I: IntoIterator<Item = HashBytes>,
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

    pub fn get_contract_state(&self, account: &HashBytes) -> Result<Option<ShardAccount>> {
        let cache = self.sc_accounts.read();
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
        shard_state: ShardStateStuff,
    ) -> Result<()> {
        let shard_accounts = shard_state.state().load_accounts()?;
        let state_handle = shard_state.ref_mc_state_handle().clone();

        let mut shards_accounts = self.sc_accounts.write();
        shards_accounts.insert(
            shard_id,
            CachedAccounts {
                accounts: shard_accounts,
                state_handle,
            },
        );

        Ok(())
    }

    fn handle_masterchain_block(&self, block_stuff: &BlockStuff) -> Result<()> {
        let block = block_stuff.block();
        let block_info = block.load_info()?;
        let gen_utime = block_info.gen_utime;
        self.current_utime.store(gen_utime, Ordering::Release);
        if block_info.key_block {
            let key_block = block_stuff.block();
            self.update_signature_id(key_block)?;
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
        block: &BlockStuff,
        block_hash: &HashBytes,
        shard_state_stuff: &ShardStateStuff,
    ) -> Result<FuturesUnordered<HandleTransactionStatusRx>> {
        let block_info = block.load_info()?;
        let extra = block.load_extra()?;
        let account_blocks = extra.account_blocks.load()?;
        let shard = block.id().shard;

        {
            let shard_accounts = shard_state_stuff.state().load_accounts()?;
            let state_handle = shard_state_stuff.ref_mc_state_handle().clone();

            let mut cache = self.sc_accounts.write();
            cache.insert(
                block_info.shard,
                CachedAccounts {
                    accounts: shard_accounts,
                    state_handle,
                },
            );
            if block_info.after_merge || block_info.after_split {
                tracing::debug!("clearing shard states cache after shards merge/split");

                match block_info.load_prev_ref()? {
                    // Block after split
                    //       |
                    //       *  - block A
                    //      / \
                    //     *   *  - blocks B', B"
                    PrevBlockRef::Single(..) => {
                        // Compute parent shard of the B' or B"
                        let parent = shard
                            .merge()
                            .ok_or(everscale_types::error::Error::InvalidData)?;

                        let opposite = shard.opposite().expect("after split");

                        // Remove parent shard state
                        if cache.contains_key(&shard) && cache.contains_key(&opposite) {
                            cache.remove(&parent);
                        }
                    }

                    // Block after merge
                    //     *   *  - blocks A', A"
                    //      \ /
                    //       *  - block B
                    //       |
                    PrevBlockRef::AfterMerge { .. } => {
                        // Compute parent shard of the B' or B"
                        let (left, right) = shard
                            .split()
                            .ok_or(everscale_types::error::Error::InvalidData)?;

                        // Find and remove all parent shards
                        cache.remove(&left);
                        cache.remove(&right);
                    }
                }
            }
            drop(cache);
        }

        let mut states = FuturesUnordered::new();

        let state_subscriptions = self.state_subscriptions.read();
        let token_subscription = self.token_subscription.read();
        let shards_accounts_cache = self.sc_accounts.read();

        for account_block in account_blocks.iter() {
            let (account, _, account_block) = account_block?;
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
            }
        }

        self.messages_queue
            .update(&block_info.shard, block_info.gen_utime);

        Ok(states)
    }

    fn update_signature_id(&self, key_block: &Block) -> Result<()> {
        let extra = key_block.load_extra()?;
        let custom = extra
            .load_custom()?
            .context("McBlockExtra not found in the masterchain block")?;
        let config = custom.config.context("Config not found in the key block")?;

        self.signature_id.store(
            config.get_global_version()?.capabilities.into_inner(),
            key_block.global_id,
        );

        Ok(())
    }

    async fn wait_sync(&self) {
        if self.ready.load(Ordering::Acquire) {
            return;
        }
        self.ready_signal.notified().await;
    }
}

impl TonSubscriber {
    async fn process_block(&self, ctx: StateSubscriberContext) -> Result<()> {
        let block_stuff = ctx.block;
        let block_id = block_stuff.id();

        if block_id.is_masterchain() {
            self.handle_masterchain_block(&block_stuff)?;
        } else {
            let mut states =
                self.handle_shard_block(&block_stuff, &block_id.root_hash, &ctx.state)?;
            while let Some(status) = states.next().await {
                if let Err(err) = status {
                    log::error!("Failed to receive transaction status: {}", err);
                }
            }
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
        block_info: &BlockInfo,
        account_block: &AccountBlock,
        account: &HashBytes,
        block_hash: &HashBytes,
    ) -> Result<FuturesUnordered<HandleTransactionStatusRx>> {
        let states = FuturesUnordered::new();

        if self.transaction_subscriptions.is_empty() {
            return Ok(states);
        }

        for transaction in account_block.transactions.iter() {
            let result = transaction.and_then(|(_, _, value)| {
                let hash = *value.repr_hash();
                value.load().map(|tx| (tx, hash))
            });
            let (transaction, hash) = match result {
                Ok((tx, transaction_hash)) => (tx, transaction_hash),
                Err(e) => {
                    log::error!(
                        "Failed to parse transaction in block {} for account {}: {:?}",
                        block_info.seqno,
                        account.to_string(),
                        e
                    );
                    continue;
                }
            };
            // Skip non-ordinary transactions
            let transaction_info = match transaction.load_info()? {
                TxInfo::Ordinary(info) => info,
                _ => continue,
            };

            let in_msg_hash = transaction
                .in_msg
                .as_ref()
                .map(|in_msg| *in_msg.repr_hash());
            let in_msg = match transaction.load_in_msg()? {
                Some(message) => {
                    if matches!(message.info, MsgInfo::ExtIn(_)) {
                        messages_queue.deliver_message(*account, in_msg_hash.unwrap());
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
                            hash.to_string(),
                            account.to_string(),
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
        state_subscriptions: &RwLockReadGuard<FxHashMap<HashBytes, StateSubscription>>,
        shards_accounts_cache: &FxHashMap<ShardIdent, CachedAccounts>,
        block_info: &BlockInfo,
        account_block: &AccountBlock,
        account: &HashBytes,
        block_hash: &HashBytes,
    ) -> Result<FuturesUnordered<HandleTransactionStatusRx>> {
        let states = FuturesUnordered::new();

        for transaction in account_block.transactions.iter() {
            let result = transaction.and_then(|(_, _, value)| {
                let hash = *value.repr_hash();
                value.load().map(|tx| (tx, hash))
            });
            let (transaction, hash) = match result {
                Ok((tx, transaction_hash)) => (tx, transaction_hash),
                Err(e) => {
                    log::error!(
                        "Failed to parse transaction in block {} for account {}: {:?}",
                        block_info.seqno,
                        account.to_string(),
                        e
                    );
                    continue;
                }
            };

            // Skip non-ordinary transactions
            let transaction_info = match transaction.load_info()? {
                TxInfo::Ordinary(info) => info,
                _ => continue,
            };

            let old_transaction = conver_to_old_transaction(&transaction)?;

            let old_transaction_info = match old_transaction.description.read_struct() {
                Ok(ton_block::TransactionDescr::Ordinary(info)) => info,
                _ => continue,
            };

            let parsed_token_transaction = match nekoton::core::parsing::parse_token_transaction(
                &old_transaction,
                &old_transaction_info,
                TokenWalletVersion::Tip3,
            ) {
                Some(parsed_token_transaction) => Some(parsed_token_transaction),
                None => nekoton::core::parsing::parse_token_transaction(
                    &old_transaction,
                    &old_transaction_info,
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

                if state_subscriptions
                    .get(&HashBytes::from_slice(owner_account.as_slice()))
                    .is_some()
                {
                    let in_msg = match transaction.load_in_msg()? {
                        Some(message) => message,
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
                                    hash.to_string(),
                                    account.to_string(),
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
        shard_state: Arc<ShardStateStuff>,
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
    fn handle_block(&mut self, block: &Block, block_info: &BlockInfo)
        -> Result<BlockAwaiterAction>;
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
    data: Cell,
    last_transaction_id: LastTransactionId,
    _state_handle: RefMcStateHandle,
}

pub fn make_existing_contract(state: Option<ShardAccount>) -> Result<Option<ExistingContract>> {
    let state = match state {
        Some(this) => this,
        None => return Ok(None),
    };

    let account = everscale_types::models::Account::load_from(&mut state.data.as_slice()?)?;

    if let Some(stuff) = convert_to_old_account(account)? {
        Ok(Some(ExistingContract {
            account: stuff,
            timings: GenTimings::Unknown,
            last_transaction_id: state.last_transaction_id,
        }))
    } else {
        Ok(None)
    }
}

pub fn convert_to_old_account(
    account: everscale_types::models::Account,
) -> Result<Option<ton_block::AccountStuff>> {
    match account.state {
        AccountState::Uninit | AccountState::Frozen(_) => Ok(None),
        AccountState::Active(state_init) => {
            Ok(Some(ton_block::AccountStuff {
                addr: ton_block::MsgAddressInt::from_str(&account.address.to_string())?,
                storage_stat: ton_block::StorageInfo {
                    used: ton_block::StorageUsed::with_values_checked(
                        account.storage_stat.used.cells.into_inner(),
                        account.storage_stat.used.bits.into_inner(),
                        account.storage_stat.used.public_cells.into_inner(),
                    )?,
                    last_paid: account.storage_stat.last_paid,
                    due_payment: account
                        .storage_stat
                        .due_payment
                        .map(|v| (v.into_inner() as u64).into()),
                },
                storage: ton_block::AccountStorage {
                    last_trans_lt: account.last_trans_lt,
                    balance: ton_block::CurrencyCollection {
                        grams: (account.balance.tokens.into_inner() as u64).into(),
                        other: ExtraCurrencyCollection::default(),
                    },
                    state: ton_block::AccountState::AccountActive {
                        state_init: ton_block::StateInit {
                            split_depth: state_init
                                .split_depth
                                .map(|v| ton_block::Number5::new(v.into_bit_len() as u32).unwrap()),
                            special: state_init.special.map(|v| ton_block::TickTock {
                                tick: v.tick,
                                tock: v.tock,
                            }),
                            code: state_init.code.map(|cell| {
                                let bytes = Boc::encode(cell);
                                ton_types::deserialize_tree_of_cells(&mut &*bytes)
                                    .unwrap_or_default()
                            }),
                            data: state_init.data.map(|cell| {
                                let bytes = Boc::encode(cell);
                                ton_types::deserialize_tree_of_cells(&mut &*bytes)
                                    .unwrap_or_default()
                            }),
                            library: Default::default(), // TODO: CHECK CONVERSION
                        },
                    },
                    init_code_hash: None,
                },
            }))
        }
    }
}

pub struct CachedAccounts {
    pub accounts: ShardAccounts,
    pub state_handle: RefMcStateHandle,
}

impl CachedAccounts {
    fn get(&self, account: &HashBytes) -> Result<Option<ShardAccount>> {
        match self.accounts.get(account)? {
            Some((_, account)) => Ok(Some(ShardAccount {
                data: account.account.as_cell().unwrap().clone(),
                last_transaction_id: LastTransactionId::Exact(TransactionId {
                    lt: account.last_trans_lt,
                    hash: UInt256::with_array(account.last_trans_hash.0),
                }),
                _state_handle: self.state_handle.clone(),
            })),
            None => Ok(None),
        }
    }
}

impl ShardAccountsMapExt for FxHashMap<ShardIdent, CachedAccounts> {
    fn find_account(&self, account: &HashBytes) -> Result<Option<ExistingContract>> {
        let item = self
            .iter()
            .find(|(shard_ident, _)| contains_account(shard_ident, account));

        match item {
            Some((_, shard)) => {
                if let Some((_, account)) = shard.accounts.get(account)? {
                    let last_transaction_id = LastTransactionId::Exact(TransactionId {
                        lt: account.last_trans_lt,
                        hash: UInt256::with_array(account.last_trans_hash.0),
                    });

                    if let Some(account) = account.load_account()? {
                        if let Some(stuff) = convert_to_old_account(account)? {
                            return Ok(Some(ExistingContract {
                                account: stuff,
                                timings: GenTimings::Unknown,
                                last_transaction_id,
                            }));
                        }
                    }
                }
                Ok(None)
            }
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
