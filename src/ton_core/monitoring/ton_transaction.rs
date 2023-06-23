use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc;
use ton_types::UInt256;

use crate::ton_core::monitoring::*;
use crate::ton_core::*;

pub struct TonTransaction {
    context: Arc<TonCoreContext>,
    ton_transaction_producer: TonTransactionTx,
    ton_transaction_observer: Arc<AccountObserver<TonTransactionEvent>>,
}

impl TonTransaction {
    pub async fn new(
        context: Arc<TonCoreContext>,
        ton_transaction_producer: TonTransactionTx,
    ) -> Result<Arc<Self>> {
        let (ton_transaction_events_tx, ton_transaction_events_rx) = mpsc::unbounded_channel();

        let ton_transaction = Arc::new(Self {
            context,
            ton_transaction_producer,
            ton_transaction_observer: AccountObserver::new(ton_transaction_events_tx),
        });

        ton_transaction.start_listening_ton_transaction_events(ton_transaction_events_rx);

        Ok(ton_transaction)
    }

    pub fn add_account_subscription<I>(&self, accounts: I)
    where
        I: IntoIterator<Item = UInt256>,
    {
        self.context
            .ton_subscriber
            .add_transactions_subscription(accounts, &self.ton_transaction_observer);
    }

    fn start_listening_ton_transaction_events(self: &Arc<Self>, mut rx: TonTransactionEventsRx) {
        let ton_transaction = Arc::downgrade(self);

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let ton_transaction = match ton_transaction.upgrade() {
                    Some(engine) => engine,
                    None => {
                        event.state.send(HandleTransactionStatus::Fail).ok();
                        log::error!("Failed to handle received ton transaction: Ton transaction handler was dropped");
                        break;
                    }
                };

                match ton_transaction_parser::parse_ton_transaction(
                    event.account,
                    event.block_utime,
                    event.transaction_hash,
                    event.transaction,
                )
                .await
                {
                    Ok(transaction) => {
                        ton_transaction
                            .ton_transaction_producer
                            .send((transaction, event.state))
                            .ok();
                    }
                    Err(e) => {
                        event.state.send(HandleTransactionStatus::Fail).ok();
                        log::error!(
                            "Failed to handle received ton transaction `{}`: {}",
                            event.transaction_hash,
                            e
                        );
                    }
                }
            }

            rx.close();
            while rx.recv().await.is_some() {}
        });
    }
}

#[derive(Debug)]
pub struct TonTransactionEvent {
    pub account: UInt256,
    pub block_utime: u32,
    pub transaction_hash: UInt256,
    pub transaction: ton_block::Transaction,
    pub state: HandleTransactionStatusTx,
}

impl ReadFromTransaction for TonTransactionEvent {
    fn read_from_transaction(
        ctx: &TxContext<'_>,
        state: HandleTransactionStatusTx,
    ) -> Option<Self> {
        Some(TonTransactionEvent {
            account: *ctx.account,
            block_utime: ctx.block_info.gen_utime().as_u32(),
            transaction_hash: *ctx.transaction_hash,
            transaction: ctx.transaction.clone(),
            state,
        })
    }
}

type TonTransactionEventsRx = mpsc::UnboundedReceiver<TonTransactionEvent>;
