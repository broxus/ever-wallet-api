use std::sync::Arc;

use anyhow::Result;
use nekoton::core::models::*;
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

    pub fn init_token_subscription(&self) {
        self.context
            .ton_subscriber
            .add_token_subscription(&self.token_transaction_observer);
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
                        if let Err(err) = token_transaction
                            .token_transaction_producer
                            .send(transaction)
                        {
                            log::error!(
                                "Failed to send received token transaction into channel: {:?}",
                                err
                            );
                        }
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

        if let Some(parsed) = ctx.token_transaction {
            event = Some(TokenTransactionEvent {
                ctx: TokenTransactionContext {
                    account: *ctx.account,
                    block_hash: *ctx.block_hash,
                    block_utime: ctx.block_info.gen_utime().0,
                    transaction_hash: *ctx.transaction_hash,
                    transaction: ctx.transaction.clone(),
                    shard_accounts: ctx.shard_accounts.clone(),
                },
                parsed: parsed.clone(),
            })
        }

        event
    }
}

type TokenTransactionEventsRx = mpsc::UnboundedReceiver<TokenTransactionEvent>;
