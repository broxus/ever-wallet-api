use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc;
use ton_types::UInt256;

use crate::ton_core::*;

pub struct WalletNotification {
    context: Arc<TonCoreContext>,
    wallet_notification_observer: Arc<AccountObserver<WalletNotificationEvent>>,
}

impl WalletNotification {
    pub async fn new(context: Arc<TonCoreContext>) -> Result<Arc<Self>> {
        let (wallet_notification_events_tx, wallet_notification_events_rx) =
            mpsc::unbounded_channel();

        let ton_transaction = Arc::new(Self {
            context,
            wallet_notification_observer: AccountObserver::new(wallet_notification_events_tx),
        });

        ton_transaction.start_listening_wallet_notification_events(wallet_notification_events_rx);

        Ok(ton_transaction)
    }

    pub fn add_account_subscription<I>(&self, accounts: I)
    where
        I: IntoIterator<Item = UInt256>,
    {
        self.context
            .ton_subscriber
            .add_transactions_subscription(accounts, &self.wallet_notification_observer);
    }

    fn start_listening_wallet_notification_events(
        self: &Arc<Self>,
        mut rx: WalletNotificationsEventsRx,
    ) {
        let ton_transaction = Arc::downgrade(self);

        tokio::spawn(async move {
            while let Some(_event) = rx.recv().await {
                let _ton_transaction = match ton_transaction.upgrade() {
                    Some(engine) => engine,
                    None => break,
                };

                todo!()
            }

            rx.close();
            while rx.recv().await.is_some() {}
        });
    }
}

#[derive(Debug)]
pub struct WalletNotificationEvent {
    pub account: UInt256,
    pub notify_wallet_deployed: NotifyWalletDeployed,
}

impl ReadFromTransaction for WalletNotificationEvent {
    fn read_from_transaction(ctx: &TxContext<'_>) -> Option<Self> {
        let notify_wallet_deployed = notify_wallet_deployed();

        let in_msg_body = ctx.in_msg_internal()?.body()?;
        let event: Option<NotifyWalletDeployed> =
            match nekoton_abi::read_function_id(&in_msg_body).ok()? {
                id if id == notify_wallet_deployed.input_id => {
                    match notify_wallet_deployed
                        .decode_input(in_msg_body.clone(), true)
                        .and_then(|tokens| tokens.unpack().map_err(anyhow::Error::from))
                    {
                        Ok(parsed) => Some(parsed),
                        Err(e) => {
                            log::error!("Failed to parse wallet deployed event: {:?}", e);
                            None
                        }
                    }
                }
                _ => None,
            };

        if let Some(event) = event {
            return Some(WalletNotificationEvent {
                account: *ctx.account,
                notify_wallet_deployed: event,
            });
        }

        None
    }
}

type WalletNotificationsEventsRx = mpsc::UnboundedReceiver<WalletNotificationEvent>;
