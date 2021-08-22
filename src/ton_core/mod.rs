use std::collections::{hash_map, HashMap};
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use nekoton::core::models::RootTokenContractDetails;
use nekoton::core::token_wallet::{RootTokenContractState, TokenWalletContractState};
use nekoton_abi::LastTransactionId;
use parking_lot::Mutex;
use serde::Deserialize;
use tokio::sync::mpsc;
use ton_block::{MsgAddressInt, Serializable};
use ton_types::UInt256;

use self::models::*;
use self::ton_subscriber::*;

use crate::models::owners_cache::*;
use crate::models::token_transactions::*;
use crate::models::transactions::*;

mod models;
mod ton_subscriber;

pub const TOKEN_WALLET_CODE_HASH: [u8; 32] = [
    44, 127, 188, 81, 97, 200, 223, 145, 75, 25, 193, 126, 27, 104, 81, 113, 32, 159, 175, 201, 32,
    0, 153, 178, 193, 252, 136, 125, 89, 93, 42, 227,
];

pub struct TonCore {
    ton_engine: Arc<ton_indexer::Engine>,
    ton_subscriber: Arc<TonSubscriber>,

    account_observer: Arc<AccountObserver>,
    pending_messages: Mutex<HashMap<UInt256, u32>>,

    receive_transaction_tx: ReceiveTransactionTx,

    initialized: tokio::sync::Mutex<bool>,
}

impl TonCore {
    pub async fn new(
        config: TonCoreConfig,
        global_config: ton_indexer::GlobalConfig,
        receive_transaction_tx: ReceiveTransactionTx,
    ) -> Result<Arc<Self>> {
        let ton_subscriber = TonSubscriber::new();

        let ton_engine = ton_indexer::Engine::new(
            config.ton_indexer,
            global_config,
            vec![ton_subscriber.clone() as Arc<dyn ton_indexer::Subscriber>],
        )
        .await?;

        let (account_transaction_tx, account_transaction_rx) = mpsc::unbounded_channel();

        let engine = Arc::new(Self {
            ton_engine,
            ton_subscriber,
            account_observer: Arc::new(AccountObserver {
                tx: account_transaction_tx,
            }),
            pending_messages: Mutex::new(HashMap::new()),
            receive_transaction_tx,
            initialized: Default::default(),
        });

        engine.start_listening_accounts_transactions(account_transaction_rx);
        engine.start_pending_messages_watcher();

        Ok(engine)
    }

    pub async fn start(&self) -> Result<()> {
        let mut initialized = self.initialized.lock().await;
        if *initialized {
            return Err(EngineError::AlreadyInitialized.into());
        }

        self.ton_engine.start().await?;
        self.ton_subscriber.start().await?;

        *initialized = true;
        Ok(())
    }

    pub async fn get_ton_address_info(&self, account: UInt256) -> Result<TonAddressInfo> {
        let contract = match self.ton_subscriber.get_contract_state(account).await? {
            Some(contract) => contract,
            None => return Err(EngineError::AccountNotFound(account.to_hex_string()).into()),
        };

        let workchain_id = contract.account.addr.workchain_id();
        let hex = contract.account.addr.address().to_hex_string();
        let account_status = contract.account.storage.state;
        let network_balance = contract.account.storage.balance.grams.0;

        let mut last_transaction_hash = None;
        let mut last_transaction_lt = None;
        if let LastTransactionId::Exact(transaction_id) = contract.last_transaction_id {
            last_transaction_hash = Some(transaction_id.hash);
            last_transaction_lt = Some(transaction_id.lt)
        }

        Ok(TonAddressInfo {
            workchain_id,
            hex,
            network_balance,
            account_status,
            last_transaction_lt,
            last_transaction_hash,
        })
    }

    pub async fn get_token_address_info(&self, account: UInt256) -> Result<TokenAddressInfo> {
        let contract = match self.ton_subscriber.get_contract_state(account).await? {
            Some(contract) => contract,
            None => return Err(EngineError::AccountNotFound(account.to_hex_string()).into()),
        };

        let token_wallet = TokenWalletContractState(&contract);
        let version = token_wallet.get_version()?;
        let root_address = token_wallet.get_details(version)?.root_address;
        let network_balance = token_wallet.get_balance(version)?;

        let workchain_id = contract.account.addr.workchain_id();
        let hex = contract.account.addr.address().to_hex_string();
        let account_status = contract.account.storage.state;

        let mut last_transaction_hash = None;
        let mut last_transaction_lt = None;
        if let nekoton_abi::LastTransactionId::Exact(transaction_id) = contract.last_transaction_id
        {
            last_transaction_hash = Some(transaction_id.hash);
            last_transaction_lt = Some(transaction_id.lt)
        }

        Ok(TokenAddressInfo {
            workchain_id,
            hex,
            root_address,
            network_balance,
            account_status,
            last_transaction_lt,
            last_transaction_hash,
        })
    }

    pub async fn get_token_address(&self, owner: OwnerInfo) -> Result<MsgAddressInt> {
        let root_account = UInt256::from_be_bytes(&owner.root_address.address().get_bytestring(0));
        let root_contract = match self.ton_subscriber.get_contract_state(root_account).await? {
            Some(contract) => contract,
            None => return Err(EngineError::AccountNotFound(root_account.to_hex_string()).into()),
        };

        let state = RootTokenContractState(&root_contract);
        let RootTokenContractDetails { version, .. } = state.guess_details()?;

        state.get_wallet_address(version, &owner.owner_address, None)
    }

    pub async fn send_ton_message(
        &self,
        message: &ton_block::Message,
        expire_at: u32,
    ) -> Result<()> {
        let to = match message.header() {
            ton_block::CommonMsgInfo::ExtInMsgInfo(header) => {
                ton_block::AccountIdPrefixFull::prefix(&header.dst)?
            }
            _ => return Err(EngineError::ExternalTonMessageExpected.into()),
        };

        let cells = message.write_to_new_cell()?.into();
        let serialized = ton_types::serialize_toc(&cells)?;

        self.add_pending_message(message, expire_at)?;

        match self
            .ton_engine
            .broadcast_external_message(&to, &serialized)
            .await
        {
            Ok(()) => Ok(()),
            Err(e) => {
                self.cancel_pending_message(message)?;
                Err(e)
            }
        }
    }

    pub fn add_account_subscription<I>(&self, accounts: I)
    where
        I: IntoIterator<Item = UInt256>,
    {
        self.ton_subscriber
            .add_transactions_subscription(accounts, &self.account_observer);
    }

    fn start_listening_accounts_transactions(
        self: &Arc<Self>,
        mut rx: mpsc::UnboundedReceiver<AccountTransaction>,
    ) {
        let engine = Arc::downgrade(self);

        tokio::spawn(async move {
            while let Some(transaction) = rx.recv().await {
                let engine = match engine.upgrade() {
                    Some(engine) => engine,
                    None => break,
                };

                log::info!("Transaction: {:#?}", transaction);

                // Temp example to find token
                /*if TOKEN_WALLET_CODE_HASH.as_ref() == {
                    let contract = engine
                        .ton_subscriber
                        .get_contract_state(transaction.account)
                        .await
                        .unwrap()
                        .unwrap();

                    let state = TokenWalletContractState(&contract);

                    state.get_code_hash().unwrap().as_slice()
                } {}*/

                if let Some(in_msg) = transaction
                    .transaction
                    .in_msg
                    .as_ref()
                    .and_then(|data| data.read_struct().ok())
                {
                    if let ton_block::CommonMsgInfo::ExtInMsgInfo(header) = in_msg.header() {
                        match engine.cancel_pending_message(&in_msg) {
                            Ok(()) => {
                                /*if !token {
                                    let transaction = UpdateSentTransaction {};
                                    engine.receive_transaction_tx.send(
                                        ReceiveTransaction::UpdateSent(transaction),
                                    );
                                } else {
                                    let transaction = UpdateSentTokenTransaction {};
                                    engine.receive_transaction_tx.send(
                                        ReceiveTransaction::UpdateSentToken(transaction),
                                    );
                                }*/
                            }
                            Err(e) => {
                                // TODO: somehow handle if received message not exist
                            }
                        };
                    } else {
                        /*if !token {
                            let transaction = CreateReceiveTransaction {};
                            engine.receive_transaction_tx.send(
                                ReceiveTransaction::Create(transaction),
                            );
                        } else {
                            let transaction = CreateReceiveTokenTransaction {};
                            engine.receive_transaction_tx.send(
                                ReceiveTransaction::CreateToken(transaction),
                            );
                        }*/
                    }
                }
            }

            rx.close();
            while rx.recv().await.is_some() {}
        });
    }

    fn add_pending_message(&self, message: &ton_block::Message, expire_at: u32) -> Result<()> {
        let mut msg_cache = self.pending_messages.lock();
        let msg_hash = message.serialize()?.repr_hash();
        match msg_cache.entry(msg_hash) {
            hash_map::Entry::Vacant(entry) => {
                entry.insert(expire_at);
            }
            hash_map::Entry::Occupied(_) => {
                return Err(EngineError::PendingMessageExist(msg_hash.to_hex_string()).into());
            }
        };
        Ok(())
    }

    fn cancel_pending_message(&self, message: &ton_block::Message) -> Result<()> {
        let mut msg_cache = self.pending_messages.lock();
        let msg_hash = message.serialize()?.repr_hash();
        if msg_cache
            .remove(&message.serialize()?.repr_hash())
            .is_none()
        {
            return Err(EngineError::PendingMessageNotExist(msg_hash.to_hex_string()).into());
        }

        Ok(())
    }

    fn start_pending_messages_watcher(self: &Arc<Self>) {
        let engine = Arc::downgrade(self);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(1000));

            while let Some(engine) = engine.upgrade() {
                interval.tick().await;

                let now = Utc::now().timestamp() as u32;

                let mut msg_cache = engine.pending_messages.lock();
                msg_cache.retain(|msg_hash, expire_at| {
                    let expired = now > *expire_at;
                    if expired {
                        // TODO: mark send transaction as expired
                        log::warn!("Transaction expired: {:#?}", msg_hash);
                    }
                    !expired
                });
            }
        });
    }
}

#[derive(Debug)]
struct AccountTransaction {
    account: UInt256,
    transaction_hash: UInt256,
    transaction: ton_block::Transaction,
}

struct AccountObserver {
    tx: mpsc::UnboundedSender<AccountTransaction>,
}

impl TransactionsSubscription for AccountObserver {
    fn handle_transaction(&self, ctx: TxContext<'_>) -> Result<()> {
        let transaction = AccountTransaction {
            account: *ctx.account,
            transaction_hash: *ctx.transaction_hash,
            transaction: ctx.transaction.clone(),
        };

        self.tx.send(transaction)?;

        // Done
        Ok(())
    }
}

#[derive(Deserialize, Clone)]
pub struct TonCoreConfig {
    pub ton_indexer: ton_indexer::NodeConfig,
}

pub enum ReceiveTransaction {
    Create(CreateReceiveTransaction),
    CreateToken(CreateReceiveTokenTransaction),
    UpdateSent(UpdateSentTransaction),
    UpdateSentToken(UpdateSentTokenTransaction),
}

pub type ReceiveTransactionTx = mpsc::UnboundedSender<ReceiveTransaction>;
pub type ReceiveTransactionRx = mpsc::UnboundedReceiver<ReceiveTransaction>;

#[derive(thiserror::Error, Debug)]
enum EngineError {
    #[error("Already initialized")]
    AlreadyInitialized,
    #[error("External ton message expected")]
    ExternalTonMessageExpected,
    #[error("Pending message hash `{0}` exist")]
    PendingMessageExist(String),
    #[error("Pending message hash `{0}` not exist")]
    PendingMessageNotExist(String),
    #[error("Account `{0}` not found")]
    AccountNotFound(String),
}
