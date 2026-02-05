use std::collections::hash_map;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Weak};

use anyhow::Result;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use nekoton::core::models::TokenWalletVersion;
use nekoton_utils::TrustMe;
use parking_lot::{Mutex, RwLock, RwLockReadGuard};
use rustc_hash::FxHashMap;
use tycho_types::boc::Boc;
use tycho_types::cell::{Cell, CellBuilder, HashBytes, Load};

use ton_block::Deserializable;
use ton_types::SliceData;
use tycho_block_util::block::BlockStuff;
use tycho_block_util::state::{RefMcStateHandle, ShardStateStuff};
use tycho_vm::StackValue;

use crate::models::ExistingContract;
use crate::ton_core::*;

pub struct TonSubscriber {
    // tip block timestamp
    current_utime: AtomicU32,
    signature_id: SignatureId,
    state_subscriptions: RwLock<FxHashMap<HashBytes, StateSubscription>>,
    token_subscription: RwLock<Option<TokenSubscription>>,
    sc_accounts: RwLock<FxHashMap<ShardIdent, CachedAccounts>>,
    mc_block_awaiters: Mutex<FxHashMap<usize, Box<dyn BlockAwaiter>>>,
    messages_queue: Arc<PendingMessagesQueue>,
}

impl TonSubscriber {
    pub fn new(messages_queue: Arc<PendingMessagesQueue>) -> Arc<Self> {
        Arc::new(Self {
            current_utime: AtomicU32::new(0),
            signature_id: SignatureId::default(),
            state_subscriptions: RwLock::new(FxHashMap::with_capacity_and_hasher(
                1024,
                Default::default(),
            )),
            token_subscription: Default::default(),
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
            ready: true,
            current_utime: self.current_utime(),
            signature_id: self.signature_id(),
            pending_message_count: self.messages_queue.len(),
        }
    }

    pub async fn start(self: &Arc<Self>, capabilities: u64, global_id: i32) -> Result<()> {
        self.update_signature_id(capabilities, global_id)?;
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
                        transaction_subscription: weak.clone(),
                    });
                }
                hash_map::Entry::Occupied(mut entry) => {
                    entry.get_mut().transaction_subscription = weak.clone();
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

        let mut mc_block_awaiters = self.mc_block_awaiters.lock();
        mc_block_awaiters.retain(
            |_, awaiter| match awaiter.handle_block(block, &block_info) {
                Ok(action) => action == BlockAwaiterAction::Retain,
                Err(e) => {
                    tracing::error!("Failed to handle masterchain block: {:?}", e);
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
                            .ok_or(tycho_types::error::Error::InvalidData)?;

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
                            .ok_or(tycho_types::error::Error::InvalidData)?;

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
                        block_info,
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
                            tracing::error!("Failed to handle block: {:?}", e);
                        }
                    };
                }
                None => {
                    let token_subscription = token_subscription.as_ref().trust_me();

                    match token_subscription.handle_block(
                        &state_subscriptions,
                        &shards_accounts_cache,
                        block_info,
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
                            tracing::error!("Failed to handle block: {:?}", e);
                        }
                    }
                }
            }
        }

        self.messages_queue
            .update(&block_info.shard, block_info.gen_utime);

        Ok(states)
    }

    fn update_signature_id(&self, capabilities: u64, global_id: i32) -> Result<()> {
        self.signature_id.store(capabilities, global_id);
        Ok(())
    }
}

impl TonSubscriber {
    pub async fn process_block(
        &self,
        block_stuff: &BlockStuff,
        state: &ShardStateStuff,
    ) -> Result<()> {
        let block_id = block_stuff.id();

        if block_id.is_masterchain() {
            self.handle_masterchain_block(block_stuff)?;
        } else {
            let mut states = self.handle_shard_block(block_stuff, &block_id.root_hash, state)?;
            while let Some(status) = states.next().await {
                if let Err(err) = status {
                    tracing::error!("Failed to receive transaction status: {}", err);
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
    transaction_subscription: Weak<dyn TransactionsSubscription>,
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

        for transaction in account_block.transactions.iter() {
            let result = transaction.and_then(|(_, _, value)| {
                let hash = *value.repr_hash();
                value.load().map(|tx| (tx, hash))
            });
            let (transaction, transaction_hash) = match result {
                Ok((tx, transaction_hash)) => (tx, transaction_hash),
                Err(e) => {
                    tracing::error!(
                        "Failed to parse transaction in block {} for account {}: {:?}",
                        block_info.seqno,
                        account.to_string(),
                        e
                    );
                    continue;
                }
            };

            let account = *account;
            let block_hash = *block_hash;
            // Skip non-ordinary transactions
            let transaction_info = match transaction.load_info() {
                Ok(TxInfo::Ordinary(info)) => info,
                _ => continue,
            };

            let in_msg = match  transaction.in_msg.as_ref()
            {
                Some(in_msg_cell) => {
                    let in_msg =  OwnedMessage::load_from(&mut in_msg_cell.as_slice()?)?;
                    if matches!(in_msg.info, MsgInfo::ExtIn(_)) {
                        messages_queue.deliver_message(
                            account,
                            *CellBuilder::build_from(&in_msg)?.repr_hash(),
                        );
                    }
                    in_msg
                }
                _ => continue,
            };


            let ctx = TxContext {
                block_info_gen_utime: block_info.gen_utime,
                block_hash: &block_hash,
                account: &account,
                transaction_hash: &transaction_hash,
                transaction_info: &transaction_info,
                transaction: &transaction,
                in_msg: &in_msg,
                token_transaction: &None,
                token_state: &None,
            };

            // Handle transaction
            let (tx, rx) = oneshot::channel();
            if let Some(subscription) = self.transaction_subscription.upgrade() {
                match subscription.handle_transaction(ctx, tx) {
                    Ok(_) => {
                        states.push(rx);
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to handle transaction {} for account {}: {:?}",
                            transaction_hash.to_string(),
                            account.to_string(),
                            e
                        );
                    }
                };
            }
        }

        Ok(states)
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
            let (transaction, transaction_hash) = match result {
                Ok((tx, transaction_hash)) => (tx, transaction_hash),
                Err(e) => {
                    tracing::error!(
                        "Failed to parse transaction in block {} for account {}: {:?}",
                        block_info.seqno,
                        account.to_string(),
                        e
                    );
                    continue;
                }
            };

            let account = *account;
            let block_hash = *block_hash;
            // Skip non-ordinary transactions
            let transaction_info = match transaction.load_info() {
                Ok(TxInfo::Ordinary(info)) => info,
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
                    .find_account(&HashBytes::from_slice(account.as_slice()))?
                    .ok_or_else(|| TonCoreError::AccountNotExist(account.to_string()))?;

                let (token_wallet_details, ..) = get_token_wallet_details(&token_contract)?;
                let owner_account = 
                    &token_wallet_details
                        .owner_address
                ;

                if state_subscriptions
                    .get(owner_account)
                    .is_some()
                {
                    let in_msg = match  transaction.in_msg.as_ref()
                    {
                        Some(in_msg_cell) => {
                            OwnedMessage::load_from(&mut in_msg_cell.as_slice()?)?
                        }
                        _ => continue,
                    };

                    let ctx = TxContext {
                        block_info_gen_utime: block_info.gen_utime,
                        block_hash: &block_hash,
                        account: &account,
                        transaction_hash: &transaction_hash,
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
                                tracing::error!(
                                    "Failed to handle token transaction {} for account {}: {:?}",
                                    transaction_hash.to_string(),
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
                tracing::error!("Failed to send event: channel is dropped");
            }
        }

        // Done
        Ok(())
    }
}

pub struct ShardAccount {
    pub(crate) data: Cell,
    pub(crate) last_transaction_hash: HashBytes,
    _state_handle: RefMcStateHandle,
}

pub fn make_existing_contract(state: Option<ShardAccount>) -> Result<Option<ExistingContract>> {
    let state = match state {
        Some(this) => this,
        None => return Ok(None),
    };

    let account = tycho_types::models::OptionalAccount::load_from(&mut state.data.as_slice()?)?;
    Ok(account.0.map(|account| ExistingContract {
        account,
        last_transaction_hash: state.last_transaction_hash,
    }))
}

pub fn convert_to_old_account(
    account: tycho_types::models::OptionalAccount,
) -> Result<Option<ton_block::AccountStuff>> {
    let cell = CellBuilder::build_from(account)?;
    let bytes = Boc::encode(cell);
    let cell = ton_types::deserialize_tree_of_cells(&mut &*bytes)?;
    match ton_block::Account::construct_from(&mut SliceData::load_cell(cell)?)? {
        ton_block::Account::AccountNone => Ok(None),
        ton_block::Account::Account(stuff) => Ok(Some(stuff)),
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
                last_transaction_hash: account.last_trans_hash,
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
                    let last_transaction_hash = account.last_trans_hash;
                    let account = account.account.load()?;
                    if let Some(account) = account.0 {
                        return Ok(Some(ExistingContract {
                            account,
                            last_transaction_hash,
                        }));
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
