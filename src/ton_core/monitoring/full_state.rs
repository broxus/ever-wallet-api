/*use std::sync::Arc;

use anyhow::Result;
use nekoton::core::models::*;
use tokio::sync::mpsc;
use ton_types::UInt256;

use crate::ton_core::monitoring::*;
use crate::ton_core::*;

pub struct FullState {
    context: Arc<TonCoreContext>,
    full_state_events_tx: FullStateTx,
}

impl FullState {
    pub async fn new(context: Arc<TonCoreContext>) -> Result<Arc<Self>> {
        let (full_state_events_tx, full_state_events_rx) = mpsc::unbounded_channel();

        let full_state = Arc::new(Self {
            context,
            full_state_events_tx,
        });

        full_state.start_listening_full_state_events(full_state_events_rx);

        Ok(full_state)
    }

    fn start_listening_full_state_events(self: &Arc<Self>, mut rx: FullStateRx) {
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

                match parse_token_transaction(
                    event.ctx,
                    event.parsed,
                    &token_transaction.context.sqlx_client,
                    &token_transaction.context.owners_cache,
                    &token_transaction.context.states_cache,
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

        if let Some(parsed) = ctx.token_transaction {
            event = Some(TokenTransactionEvent {
                ctx: TokenTransactionContext {
                    account: *ctx.account,
                    block_hash: *ctx.block_hash,
                    block_utime: ctx.block_info.gen_utime().0,
                    transaction_hash: *ctx.transaction_hash,
                    transaction: ctx.transaction.clone(),
                },
                parsed: parsed.clone(),
                state,
            })
        }

        event
    }
}

type TokenTransactionEventsRx = mpsc::UnboundedReceiver<TokenTransactionEvent>;
*/
