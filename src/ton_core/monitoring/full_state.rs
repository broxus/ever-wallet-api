use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc;

use crate::ton_core::*;

pub struct FullState {
    context: Arc<TonCoreContext>,
    full_state_observer: Arc<AccountObserver<FullStateEvent>>,
}

impl FullState {
    pub async fn new(context: Arc<TonCoreContext>) -> Result<Arc<Self>> {
        let (full_state_events_tx, full_state_events_rx) = mpsc::unbounded_channel();

        let full_state = Arc::new(Self {
            context,
            full_state_observer: AccountObserver::new(full_state_events_tx),
        });

        full_state.start_listening_full_state_events(full_state_events_rx);

        Ok(full_state)
    }

    pub fn init_full_state_subscription(&self) {
        self.context
            .ton_subscriber
            .add_full_state_subscription(&self.full_state_observer);
    }

    fn start_listening_full_state_events(self: &Arc<Self>, mut rx: FullStateEventsRx) {
        let full_state = Arc::downgrade(self);

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let full_state = match full_state.upgrade() {
                    Some(engine) => engine,
                    None => {
                        event.state.send(HandleTransactionStatus::Fail).ok();
                        log::error!("Failed to handle full state: Full state handler was dropped");
                        break;
                    }
                };

                let sqlx_client = &full_state.context.sqlx_client;
                match sqlx_client
                    .create_last_key_block(&event.block_id.to_string())
                    .await
                {
                    Ok(_) => event.state.send(HandleTransactionStatus::Success).ok(),
                    Err(_) => event.state.send(HandleTransactionStatus::Fail).ok(),
                };
            }

            rx.close();
            while rx.recv().await.is_some() {}
        });
    }
}

pub struct FullStateEvent {
    block_id: ton_block::BlockIdExt,
    state: HandleTransactionStatusTx,
}

impl ReadFromState for FullStateEvent {
    fn read_from_state(ctx: &StateContext<'_>, state: HandleTransactionStatusTx) -> Self {
        FullStateEvent {
            block_id: ctx.block_id.clone(),
            state,
        }
    }
}

type FullStateEventsRx = mpsc::UnboundedReceiver<FullStateEvent>;
