use std::sync::Arc;

use anyhow::Result;
use nekoton::core::models::*;
use nekoton_utils::TrustMe;
use tokio::sync::mpsc;
use ton_types::UInt256;

use crate::ton_core::monitoring::*;
use crate::ton_core::*;

pub struct TokenTransaction {
    context: Arc<TonCoreContext>,
    token_transaction_producer: CaughtTokenTransactionTx,
    token_transaction_observer: Arc<AccountObserver<TokenTransactionEvent>>,
}

impl TokenTransaction {
    pub async fn new(
        context: Arc<TonCoreContext>,
        token_transaction_producer: CaughtTokenTransactionTx,
    ) -> Result<Arc<Self>> {
        let (token_transaction_events_tx, token_transaction_events_rx) = mpsc::unbounded_channel();

        let token_transaction = Arc::new(Self {
            context,
            token_transaction_producer,
            token_transaction_observer: AccountObserver::new(token_transaction_events_tx),
        });

        token_transaction.start_listening_token_transaction_events(token_transaction_events_rx);

        Ok(token_transaction)
    }

    pub async fn init_subscriptions(&self, root_state_cache: RootStateCache) -> Result<()> {
        let owner_addresses = self
            .context
            .sqlx_client
            .get_all_addresses()
            .await?
            .into_iter()
            .map(|item| {
                nekoton_utils::repack_address(&format!("{}:{}", item.workchain_id, item.hex))
                    .trust_me()
            })
            .collect::<Vec<MsgAddressInt>>();

        let mut token_accounts = Vec::new();
        for owner_address in &owner_addresses {
            let _ = root_state_cache.iter().map(|(_, root_state)| {
                let token_account =
                    get_token_wallet_account(root_state.clone(), owner_address).trust_me();
                token_accounts.push(token_account);
            });
        }

        self.context
            .ton_subscriber
            .add_transactions_subscription(token_accounts, &self.token_transaction_observer);

        Ok(())
    }

    pub fn add_account_subscription<I>(&self, accounts: I)
    where
        I: IntoIterator<Item = UInt256>,
    {
        self.context
            .ton_subscriber
            .add_transactions_subscription(accounts, &self.token_transaction_observer);
    }

    fn start_listening_token_transaction_events(
        self: &Arc<Self>,
        mut rx: TokenTransactionEventsRx,
    ) {
        let token_transaction = Arc::downgrade(self);

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let token_transaction = match token_transaction.upgrade() {
                    Some(engine) => engine,
                    None => break,
                };

                match parse_token_transaction(
                    event.ctx,
                    event.parsed,
                    &token_transaction.context.sqlx_client,
                    &token_transaction.context.owners_cache,
                )
                .await
                {
                    Ok(transaction) => {
                        token_transaction
                            .token_transaction_producer
                            .send(transaction)
                            .ok();
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
pub struct TokenTransactionContext {
    pub account: UInt256,
    pub block_hash: UInt256,
    pub block_utime: u32,
    pub transaction_hash: UInt256,
    pub transaction: ton_block::Transaction,
    pub shard_accounts: ton_block::ShardAccounts,
}

#[derive(Debug)]
pub struct TokenTransactionEvent {
    ctx: TokenTransactionContext,
    parsed: TokenWalletTransaction,
}

impl ReadFromTransaction for TokenTransactionEvent {
    fn read_from_transaction(ctx: &TxContext<'_>) -> Option<Self> {
        let mut event = None;

        if ctx.transaction_info.aborted {
            return event;
        }

        let parsed = nekoton::core::parsing::parse_token_transaction(
            ctx.transaction,
            ctx.transaction_info,
            TokenWalletVersion::Tip3v4,
        );

        if let Some(parsed) = parsed {
            event = Some(TokenTransactionEvent {
                ctx: TokenTransactionContext {
                    account: *ctx.account,
                    block_hash: *ctx.block_hash,
                    block_utime: ctx.block_info.gen_utime().0,
                    transaction_hash: *ctx.transaction_hash,
                    transaction: ctx.transaction.clone(),
                    shard_accounts: ctx.shard_accounts.clone(),
                },
                parsed,
            })
        }

        event
    }
}

type TokenTransactionEventsRx = mpsc::UnboundedReceiver<TokenTransactionEvent>;
