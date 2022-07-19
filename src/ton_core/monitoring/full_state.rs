use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use nekoton_utils::TrustMe;
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

                if let Ok(tokens) = sqlx_client.get_token_whitelist().await {
                    for token in tokens {
                        let address = MsgAddressInt::from_str(&token.address).trust_me();
                        let account = UInt256::from_be_bytes(&address.address().get_bytestring(0));

                        match event.shard_accounts.accounts.find_account(&account) {
                            Ok(Some(state)) => {
                                if let Err(err) = sqlx_client
                                    .update_root_token_state(
                                        &token.address,
                                        serde_json::json!(state),
                                    )
                                    .await
                                {
                                    log::error!("Failed to update root token state: {:?}", err);
                                }
                            }
                            Err(_) | Ok(None) => (),
                        }
                    }
                }

                event.state.send(HandleTransactionStatus::Success).ok();
            }

            rx.close();
            while rx.recv().await.is_some() {}
        });
    }
}

pub struct FullStateEvent {
    shard_accounts: ShardAccounts,
    state: HandleTransactionStatusTx,
}

impl ReadFromState for FullStateEvent {
    fn read_from_state(ctx: &StateContext<'_>, state: HandleTransactionStatusTx) -> Option<Self> {
        let mut event = None;

        if let Ok(shard_accounts) = ctx.shard_state.state().read_accounts() {
            let shard_accounts = ShardAccounts {
                accounts: shard_accounts,
                state_handle: ctx.shard_state.ref_mc_state_handle().clone(),
            };

            event = Some(FullStateEvent {
                shard_accounts,
                state,
            })
        }

        event
    }
}

type FullStateEventsRx = mpsc::UnboundedReceiver<FullStateEvent>;
