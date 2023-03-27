use std::sync::Arc;

use anyhow::Result;
use nekoton::core::models::*;
use tokio::sync::mpsc;
use ton_types::UInt256;

use crate::ton_core::monitoring::*;
use crate::ton_core::*;

pub struct TokenTransaction {
    context: Arc<TonCoreContext>,
    token_transaction_producer: TokenTransactionTx,
    _token_transaction_observer: Arc<AccountObserver<TokenTransactionEvent>>,
}

impl TokenTransaction {
    pub async fn new(
        context: Arc<TonCoreContext>,
        token_transaction_producer: TokenTransactionTx,
    ) -> Result<Arc<Self>> {
        let (token_transaction_events_tx, token_transaction_events_rx) = mpsc::unbounded_channel();

        let observer = AccountObserver::new(token_transaction_events_tx);
        context.ton_subscriber.add_token_subscription(&observer);

        let token_transaction = Arc::new(Self {
            context,
            token_transaction_producer,
            _token_transaction_observer: observer,
        });
        token_transaction.start_listening_token_transaction_events(token_transaction_events_rx);

        Ok(token_transaction)
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
                    None => {
                        event.state.send(HandleTransactionStatus::Fail).ok();
                        log::error!("Failed to handle received token transaction: Token transaction handler was dropped");
                        break;
                    }
                };

                match token_transaction_parser::parse_token_transaction(
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
                            .send((transaction, event.state))
                            .ok();
                    }
                    Err(e) => {
                        event.state.send(HandleTransactionStatus::Fail).ok();
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
    pub token_state: ExistingContract,
    pub in_msg: ton_block::Message,
}

#[derive(Debug)]
pub struct TokenTransactionEvent {
    ctx: TokenTransactionContext,
    parsed: TokenWalletTransaction,
    state: HandleTransactionStatusTx,
}

impl ReadFromTransaction for TokenTransactionEvent {
    fn read_from_transaction(
        ctx: &TxContext<'_>,
        state: HandleTransactionStatusTx,
    ) -> Option<Self> {
        let mut event = None;

        if ctx.transaction_info.aborted {
            return event;
        }

        if let Some(parsed) = &ctx.token_transaction {
            if let Some(token_state) = &ctx.token_state {
                event = Some(TokenTransactionEvent {
                    ctx: TokenTransactionContext {
                        account: *ctx.account,
                        block_hash: *ctx.block_hash,
                        block_utime: ctx.block_info.gen_utime().0,
                        transaction_hash: *ctx.transaction_hash,
                        transaction: ctx.transaction.clone(),
                        token_state: token_state.clone(),
                        in_msg: ctx.in_msg.clone(),
                    },
                    parsed: parsed.clone(),
                    state,
                })
            }
        }

        event
    }
}

type TokenTransactionEventsRx = mpsc::UnboundedReceiver<TokenTransactionEvent>;
