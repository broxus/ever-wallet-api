use std::convert::TryInto;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use bigdecimal::BigDecimal;
use nekoton::crypto::SignedMessage;
use nekoton_utils::{repack_address, unpack_std_smc_addr, TrustMe};
use ton_block::MsgAddressInt;
use ton_types::UInt256;
use uuid::Uuid;

use crate::client::*;
use crate::models::*;
use crate::prelude::*;
use crate::sqlx_client::*;
use crate::utils::*;

#[async_trait]
pub trait TonService: Send + Sync + 'static {
    async fn create_address(
        &self,
        service_id: &ServiceId,
        input: CreateAddress,
    ) -> Result<AddressDb, ServiceError>;
    async fn check_address(&self, address: Address) -> Result<bool, ServiceError>;
    async fn get_address_balance(
        &self,
        service_id: &ServiceId,
        address: Address,
    ) -> Result<(AddressDb, NetworkAddressData), ServiceError>;
    async fn get_address_info(
        &self,
        service_id: &ServiceId,
        address: Address,
    ) -> Result<AddressDb, ServiceError>;
    async fn create_send_transaction(
        self: Arc<Self>,
        service_id: &ServiceId,
        input: TransactionSend,
    ) -> Result<TransactionDb, ServiceError>;
    async fn create_confirm_transaction(
        self: Arc<Self>,
        service_id: &ServiceId,
        input: TransactionConfirm,
    ) -> Result<TransactionDb, ServiceError>;
    async fn create_receive_transaction(
        &self,
        input: CreateReceiveTransaction,
    ) -> Result<TransactionDb, ServiceError>;
    async fn upsert_sent_transaction(
        &self,
        message_hash: String,
        account_workchain_id: i32,
        account_hex: String,
        input: UpdateSendTransaction,
    ) -> Result<TransactionDb, ServiceError>;
    async fn get_transaction_by_mh(
        &self,
        service_id: &ServiceId,
        message_hash: &str,
    ) -> Result<TransactionDb, ServiceError>;
    async fn get_transaction_by_h(
        &self,
        service_id: &ServiceId,
        transaction_hash: &str,
    ) -> Result<TransactionDb, ServiceError>;
    async fn get_transaction_by_id(
        &self,
        service_id: &ServiceId,
        id: &uuid::Uuid,
    ) -> Result<TransactionDb, ServiceError>;
    async fn get_event_by_id(
        &self,
        service_id: &ServiceId,
        id: &uuid::Uuid,
    ) -> Result<TransactionEventDb, ServiceError>;
    async fn search_transaction(
        &self,
        service_id: &ServiceId,
        payload: &TransactionsSearch,
    ) -> Result<Vec<TransactionDb>, ServiceError>;
    async fn search_events(
        &self,
        service_id: &ServiceId,
        payload: &TransactionsEventsSearch,
    ) -> Result<Vec<TransactionEventDb>, ServiceError>;
    async fn mark_event(
        &self,
        service_id: &ServiceId,
        id: &Uuid,
    ) -> Result<TransactionEventDb, ServiceError>;
    async fn mark_all_events(
        &self,
        service_id: &ServiceId,
        event_status: Option<TonEventStatus>,
    ) -> Result<Vec<TransactionEventDb>, ServiceError>;
    async fn get_tokens_transaction_by_mh(
        &self,
        service_id: &ServiceId,
        message_hash: &str,
    ) -> Result<TokenTransactionFromDb, ServiceError>;
    async fn get_tokens_transaction_by_id(
        &self,
        service_id: &ServiceId,
        id: &uuid::Uuid,
    ) -> Result<TokenTransactionFromDb, ServiceError>;
    async fn search_token_events(
        &self,
        service_id: &ServiceId,
        payload: &TokenTransactionsEventsSearch,
    ) -> Result<Vec<TokenTransactionEventDb>, ServiceError>;
    async fn mark_token_event(
        &self,
        service_id: &ServiceId,
        id: &Uuid,
    ) -> Result<TokenTransactionEventDb, ServiceError>;
    async fn get_token_address_balance(
        &self,
        service_id: &ServiceId,
        address: &Address,
    ) -> Result<Vec<(TokenBalanceFromDb, NetworkTokenAddressData)>, ServiceError>;
    async fn create_send_token_transaction(
        self: Arc<Self>,
        service_id: &ServiceId,
        input: &TokenTransactionSend,
    ) -> Result<TransactionDb, ServiceError>;
    async fn create_burn_token_transaction(
        self: Arc<Self>,
        service_id: &ServiceId,
        input: &TokenTransactionBurn,
    ) -> Result<TransactionDb, ServiceError>;
    async fn create_mint_token_transaction(
        self: Arc<Self>,
        service_id: &ServiceId,
        input: &TokenTransactionMint,
    ) -> Result<TransactionDb, ServiceError>;
    async fn create_receive_token_transaction(
        &self,
        input: CreateTokenTransaction,
    ) -> Result<TokenTransactionFromDb, ServiceError>;
    async fn get_metrics(&self) -> Result<Metrics, ServiceError>;
}

#[derive(Clone)]
pub struct TonServiceImpl {
    sqlx_client: SqlxClient,
    request_count: Arc<RequestCount>,
    ton_api_client: Arc<dyn TonClient>,
    callback_client: Arc<dyn CallbackClient>,
    key: Arc<Vec<u8>>,
}

impl TonServiceImpl {
    pub fn new(
        sqlx_client: SqlxClient,
        ton_api_client: Arc<dyn TonClient>,
        callback_client: Arc<dyn CallbackClient>,
        key: Vec<u8>,
    ) -> Self {
        let request_count = Arc::new(RequestCount::default());
        let key = Arc::new(key);
        Self {
            sqlx_client,
            request_count,
            ton_api_client,
            callback_client,
            key,
        }
    }

    pub fn metrics(&self) -> ClientServiceMetrics {
        ClientServiceMetrics {
            create_address_count: self.request_count.create_address.load(Ordering::Acquire),
            send_transaction_count: self.request_count.send_transaction.load(Ordering::Acquire),
            recv_transaction_count: self.request_count.recv_transaction.load(Ordering::Acquire),
            send_token_transaction_count: self
                .request_count
                .send_token_transaction
                .load(Ordering::Acquire),
            recv_token_transaction_count: self
                .request_count
                .recv_token_transaction
                .load(Ordering::Acquire),
        }
    }

    pub async fn start(self: &Arc<Self>) -> Result<(), ServiceError> {
        // Load unprocessed sent messages
        let transactions: Vec<TransactionDb> = self
            .sqlx_client
            .get_all_transactions_by_status(TonTransactionStatus::New)
            .await?;

        for transaction in transactions {
            let account = UInt256::from_be_bytes(
                &hex::decode(transaction.account_hex.clone())
                    .map_err(|err| ServiceError::Other(err.into()))?,
            );
            let message_hash = UInt256::from_be_bytes(
                &hex::decode(transaction.message_hash.clone())
                    .map_err(|err| ServiceError::Other(err.into()))?,
            );
            let expire_at = transaction.created_at.timestamp() as u32 + DEFAULT_EXPIRATION_TIMEOUT;

            let rx = self
                .ton_api_client
                .add_pending_message(account, message_hash, expire_at)?;

            let ton_service = Arc::downgrade(self);
            tokio::spawn(async move {
                match rx.await {
                    Ok(MessageStatus::Delivered) => {
                        log::info!("Successfully sent message `{}`", transaction.message_hash)
                    }
                    Ok(MessageStatus::Expired) => {
                        let ton_service = match ton_service.upgrade() {
                            Some(ton_service) => ton_service,
                            None => {
                                log::error!("TonServiceImpl is already dropped");
                                return;
                            }
                        };

                        if let Err(err) = ton_service
                            .upsert_sent_transaction(
                                transaction.message_hash.clone(),
                                transaction.account_workchain_id,
                                transaction.account_hex.clone(),
                                UpdateSendTransaction::error("Expired".to_string()),
                            )
                            .await
                        {
                            log::error!(
                                "Failed to upsert expired message `{}` for reason: {:?}",
                                message_hash.to_hex_string(),
                                err
                            )
                        }
                    }
                    Err(err) => {
                        log::error!(
                            "Failed to get pending message `{}` for reason: {:?}",
                            message_hash,
                            err
                        )
                    }
                }
            });
        }

        Ok(())
    }

    async fn notify(
        &self,
        service_id: &ServiceId,
        payload: AccountTransactionEvent,
        notify_type: NotifyType,
    ) -> Result<(), ServiceError> {
        if let Ok(url) = self.sqlx_client.get_callback(*service_id).await {
            let secret = self
                .sqlx_client
                .get_key_by_service_id(service_id)
                .await
                .map(|k| k.secret)
                .map_err(|err| ServiceError::Other(err.into()))?;
            let event_status = match self
                .callback_client
                .send(url.clone(), payload.clone(), secret)
                .await
            {
                Err(e) => {
                    log::error!(
                        "Error on callback sending to {} with payload: {:#?} - {:?}",
                        url,
                        payload,
                        e
                    );
                    TonEventStatus::Error
                }
                Ok(_) => TonEventStatus::Notified,
            };
            match notify_type {
                NotifyType::Transaction => {
                    if let Err(e) = self
                        .sqlx_client
                        .update_event_status_of_transaction_event(
                            payload.message_hash.clone(),
                            payload.account.workchain_id,
                            payload.account.hex.0.clone(),
                            event_status,
                        )
                        .await
                    {
                        log::error!("Error on update event status of transaction event sending with payload: {:#?} , event status: {:#?} - {:?}", payload, event_status, e);
                    }
                }
                NotifyType::TokenTransaction => {
                    if let Err(e) = self
                        .sqlx_client
                        .update_event_status_of_token_transaction_event(
                            payload.message_hash.clone(),
                            payload.account.workchain_id,
                            payload.account.hex.0.clone(),
                            event_status,
                        )
                        .await
                    {
                        log::error!("Error on update event status of token transaction event sending with payload: {:#?} , event status: {:#?} - {:?}", payload, event_status, e);
                    }
                }
            }
        }

        Ok(())
    }

    async fn send_transaction_helper(
        self: &Arc<Self>,
        message_hash: String,
        account_hex: String,
        account_workchain_id: i32,
        signed_message: SignedMessage,
    ) -> Result<(), ServiceError> {
        let account = UInt256::from_be_bytes(
            &hex::decode(&account_hex).map_err(|err| ServiceError::Other(err.into()))?,
        );

        let status = self
            .ton_api_client
            .send_transaction(account, signed_message)
            .await?;

        match status {
            MessageStatus::Delivered => log::info!("Successfully sent message `{}`", message_hash),
            MessageStatus::Expired => {
                self.upsert_sent_transaction(
                    message_hash,
                    account_workchain_id,
                    account_hex,
                    UpdateSendTransaction::error("Expired".to_string()),
                )
                .await?;
            }
        };

        Ok(())
    }

    async fn send_transaction(
        self: &Arc<Self>,
        message_hash: String,
        account_hex: String,
        account_workchain_id: i32,
        signed_message: SignedMessage,
        non_blocking: bool,
    ) -> Result<(), ServiceError> {
        let res = if non_blocking {
            let this = self.clone();
            self.spawn_background_task("Send transaction", async move {
                this.send_transaction_helper(
                    message_hash.clone(),
                    account_hex.clone(),
                    account_workchain_id,
                    signed_message,
                )
                .await
            });
            Ok(())
        } else {
            self.send_transaction_helper(
                message_hash,
                account_hex,
                account_workchain_id,
                signed_message,
            )
            .await
        };

        res
    }

    async fn deploy_wallet(
        self: &Arc<Self>,
        service_id: &ServiceId,
        address: &AddressDb,
        public_key: &[u8],
        private_key: &[u8],
    ) -> Result<(), ServiceError> {
        let payload = self
            .ton_api_client
            .prepare_deploy(address, public_key, private_key)
            .await?;

        if let Some((payload, signed_message)) = payload {
            let (transaction, event) = self
                .sqlx_client
                .create_send_transaction(CreateSendTransaction::new(payload, *service_id))
                .await?;

            self.send_transaction(
                transaction.message_hash.clone(),
                transaction.account_hex.clone(),
                transaction.account_workchain_id,
                signed_message,
                false,
            )
            .await?;

            self.notify(service_id, event.into(), NotifyType::Transaction)
                .await?;
        }

        Ok(())
    }

    /// Waits future in background. In case of error does nothing but logging
    fn spawn_background_task<F>(self: &Arc<Self>, name: &'static str, fut: F)
    where
        F: Future<Output = Result<(), ServiceError>> + Send + 'static,
    {
        tokio::spawn(async move {
            if let Err(e) = fut.await {
                log::error!("Failed to {}: {:?}", name, e);
            }
        });
    }
}

#[async_trait]
impl TonService for TonServiceImpl {
    async fn create_address(
        &self,
        service_id: &ServiceId,
        input: CreateAddress,
    ) -> Result<AddressDb, ServiceError> {
        self.request_count
            .create_address
            .fetch_add(1, Ordering::Relaxed);

        let id = uuid::Uuid::new_v4();
        let payload = self.ton_api_client.create_address(input).await?;

        let public_key = hex::encode(&payload.public_key);
        let private_key = encrypt_private_key(
            &payload.private_key,
            self.key.as_slice().try_into().trust_me(),
            &id,
        )
        .map_err(|err| {
            ServiceError::Other(TonServiceError::EncryptPrivateKeyError(err.to_string()).into())
        })?;

        self.sqlx_client
            .create_address(CreateAddressInDb::new(
                payload,
                id,
                *service_id,
                public_key,
                private_key,
            ))
            .await
    }

    async fn check_address(&self, address: Address) -> Result<bool, ServiceError> {
        Ok(MsgAddressInt::from_str(&address.0).is_ok()
            || (unpack_std_smc_addr(&address.0, false).is_ok())
            || (unpack_std_smc_addr(&address.0, true).is_ok()))
    }

    async fn get_address_balance(
        &self,
        service_id: &ServiceId,
        address: Address,
    ) -> Result<(AddressDb, NetworkAddressData), ServiceError> {
        let address = repack_address(&address.0)?;
        let address_db = self
            .sqlx_client
            .get_address(
                *service_id,
                address.workchain_id(),
                address.address().to_hex_string(),
            )
            .await?;
        let network = self.ton_api_client.get_address_info(&address).await?;
        Ok((address_db, network))
    }

    async fn get_address_info(
        &self,
        service_id: &ServiceId,
        address: Address,
    ) -> Result<AddressDb, ServiceError> {
        let account = repack_address(&address.0)?;
        let address = self
            .sqlx_client
            .get_address(
                *service_id,
                account.workchain_id(),
                account.address().to_hex_string(),
            )
            .await?;
        Ok(address)
    }

    async fn create_send_transaction(
        self: Arc<Self>,
        service_id: &ServiceId,
        input: TransactionSend,
    ) -> Result<TransactionDb, ServiceError> {
        self.request_count
            .send_transaction
            .fetch_add(1, Ordering::Relaxed);

        let address = repack_address(&input.from_address.0)?;

        let network = self.ton_api_client.get_address_info(&address).await?;

        for transaction_output in input.outputs.iter() {
            let (_, scale) = transaction_output.value.as_bigint_and_exponent();
            if scale != 0 {
                return Err(ServiceError::WrongInput("Invalid value".to_string()));
            }
        }

        if input
            .outputs
            .iter()
            .map(|o| o.value.clone())
            .sum::<BigDecimal>()
            >= network.network_balance
            && input.outputs.iter().all(|o| {
                o.output_type.is_none() || o.output_type == Some(TransactionSendOutputType::Normal)
            })
        {
            return Err(ServiceError::WrongInput("Insufficient balance".to_string()));
        }

        let address_db = self
            .sqlx_client
            .get_address(
                *service_id,
                address.workchain_id(),
                address.address().to_hex_string(),
            )
            .await?;

        let public_key = hex::decode(address_db.public_key.clone())
            .map_err(|err| ServiceError::Other(err.into()))?;
        let private_key = decrypt_private_key(
            &address_db.private_key,
            self.key.as_slice().try_into().trust_me(),
            &address_db.id,
        )
        .map_err(|err| {
            ServiceError::Other(TonServiceError::DecryptPrivateKeyError(err.to_string()).into())
        })?;

        if network.account_status == AccountStatus::UnInit {
            self.deploy_wallet(service_id, &address_db, &public_key, &private_key)
                .await?;
        }

        let (payload, signed_message) = self
            .ton_api_client
            .prepare_transaction(
                input,
                &public_key,
                &private_key,
                &address_db.account_type,
                &address_db.custodians,
            )
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_send_transaction(CreateSendTransaction::new(payload, *service_id))
            .await?;

        self.send_transaction(
            transaction.message_hash.clone(),
            transaction.account_hex.clone(),
            transaction.account_workchain_id,
            signed_message,
            true,
        )
        .await
        .trust_me();

        self.notify(service_id, event.into(), NotifyType::Transaction)
            .await?;

        Ok(transaction)
    }

    async fn create_confirm_transaction(
        self: Arc<Self>,
        service_id: &ServiceId,
        input: TransactionConfirm,
    ) -> Result<TransactionDb, ServiceError> {
        let address = repack_address(&input.address.0)?;

        let address_db = self
            .sqlx_client
            .get_address(
                *service_id,
                address.workchain_id(),
                address.address().to_hex_string(),
            )
            .await?;

        if address_db.account_type != AccountType::SafeMultisig {
            return Err(ServiceError::WrongInput("Invalid account type".to_string()));
        }

        let public_key = hex::decode(address_db.public_key.clone())
            .map_err(|err| ServiceError::Other(err.into()))?;
        let private_key = decrypt_private_key(
            &address_db.private_key,
            self.key.as_slice().try_into().trust_me(),
            &address_db.id,
        )
        .map_err(|err| {
            ServiceError::Other(TonServiceError::DecryptPrivateKeyError(err.to_string()).into())
        })?;

        let network = self.ton_api_client.get_address_info(&address).await?;

        if network.account_status == AccountStatus::UnInit {
            self.deploy_wallet(service_id, &address_db, &public_key, &private_key)
                .await?;
        }

        let (payload, signed_message) = self
            .ton_api_client
            .prepare_confirm_transaction(input, &public_key, &private_key)
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_send_transaction(CreateSendTransaction::new(payload, *service_id))
            .await?;

        self.send_transaction(
            transaction.message_hash.clone(),
            transaction.account_hex.clone(),
            transaction.account_workchain_id,
            signed_message,
            true,
        )
        .await
        .trust_me();

        self.notify(service_id, event.into(), NotifyType::Transaction)
            .await?;

        Ok(transaction)
    }

    async fn create_receive_transaction(
        &self,
        input: CreateReceiveTransaction,
    ) -> Result<TransactionDb, ServiceError> {
        self.request_count
            .recv_transaction
            .fetch_add(1, Ordering::Relaxed);

        let address = self
            .sqlx_client
            .get_address_by_workchain_hex(input.account_workchain_id, input.account_hex.clone())
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_receive_transaction(input, address.service_id)
            .await?;

        self.notify(&address.service_id, event.into(), NotifyType::Transaction)
            .await?;

        Ok(transaction)
    }

    async fn upsert_sent_transaction(
        &self,
        message_hash: String,
        account_workchain_id: i32,
        account_hex: String,
        input: UpdateSendTransaction,
    ) -> Result<TransactionDb, ServiceError> {
        let address = self
            .sqlx_client
            .get_address_by_workchain_hex(account_workchain_id, account_hex.clone())
            .await?;

        let (transaction, event) = if self
            .sqlx_client
            .get_sent_transaction_by_mh_account(
                address.service_id,
                message_hash.clone(),
                account_workchain_id,
                account_hex.clone(),
            )
            .await?
            .is_some()
        {
            self.sqlx_client
                .update_send_transaction(message_hash, account_workchain_id, account_hex, input)
                .await?
        } else {
            self.sqlx_client
                .create_sent_transaction(
                    address.service_id,
                    message_hash,
                    account_workchain_id,
                    account_hex,
                    input,
                )
                .await?
        };

        self.notify(&address.service_id, event.into(), NotifyType::Transaction)
            .await?;

        Ok(transaction)
    }

    async fn get_transaction_by_mh(
        &self,
        service_id: &ServiceId,
        message_hash: &str,
    ) -> Result<TransactionDb, ServiceError> {
        self.sqlx_client
            .get_transaction_by_mh(*service_id, message_hash)
            .await
    }

    async fn get_transaction_by_h(
        &self,
        service_id: &ServiceId,
        transaction_hash: &str,
    ) -> Result<TransactionDb, ServiceError> {
        self.sqlx_client
            .get_transaction_by_h(*service_id, transaction_hash)
            .await
    }

    async fn get_transaction_by_id(
        &self,
        service_id: &ServiceId,
        id: &uuid::Uuid,
    ) -> Result<TransactionDb, ServiceError> {
        self.sqlx_client
            .get_transaction_by_id(*service_id, id)
            .await
    }

    async fn get_event_by_id(
        &self,
        service_id: &ServiceId,
        id: &uuid::Uuid,
    ) -> Result<TransactionEventDb, ServiceError> {
        self.sqlx_client.get_event_by_id(*service_id, id).await
    }

    async fn search_transaction(
        &self,
        service_id: &ServiceId,
        payload: &TransactionsSearch,
    ) -> Result<Vec<TransactionDb>, ServiceError> {
        self.sqlx_client
            .get_all_transactions(*service_id, payload)
            .await
    }

    async fn search_events(
        &self,
        service_id: &ServiceId,
        payload: &TransactionsEventsSearch,
    ) -> Result<Vec<TransactionEventDb>, ServiceError> {
        self.sqlx_client
            .get_all_transaction_events(*service_id, payload)
            .await
    }

    async fn mark_event(
        &self,
        service_id: &ServiceId,
        id: &Uuid,
    ) -> Result<TransactionEventDb, ServiceError> {
        self.sqlx_client
            .update_event_status_of_transaction_event_by_id(
                *service_id,
                *id,
                TonEventStatus::Notified,
            )
            .await
    }

    async fn mark_all_events(
        &self,
        service_id: &ServiceId,
        event_status: Option<TonEventStatus>,
    ) -> Result<Vec<TransactionEventDb>, ServiceError> {
        self.sqlx_client
            .update_event_status_of_transactions_event_by_status(
                *service_id,
                event_status,
                TonEventStatus::Notified,
            )
            .await
    }

    async fn get_tokens_transaction_by_mh(
        &self,
        service_id: &ServiceId,
        message_hash: &str,
    ) -> Result<TokenTransactionFromDb, ServiceError> {
        self.sqlx_client
            .get_token_transaction_by_mh(*service_id, message_hash)
            .await
    }

    async fn get_tokens_transaction_by_id(
        &self,
        service_id: &ServiceId,
        id: &uuid::Uuid,
    ) -> Result<TokenTransactionFromDb, ServiceError> {
        self.sqlx_client
            .get_token_transaction_by_id(*service_id, id)
            .await
    }

    async fn search_token_events(
        &self,
        service_id: &ServiceId,
        payload: &TokenTransactionsEventsSearch,
    ) -> Result<Vec<TokenTransactionEventDb>, ServiceError> {
        self.sqlx_client
            .get_all_token_transaction_events(*service_id, payload)
            .await
    }

    async fn mark_token_event(
        &self,
        service_id: &ServiceId,
        id: &Uuid,
    ) -> Result<TokenTransactionEventDb, ServiceError> {
        self.sqlx_client
            .update_event_status_of_token_transaction_event_by_id(
                *service_id,
                *id,
                TonEventStatus::Notified,
            )
            .await
    }

    async fn get_token_address_balance(
        &self,
        service_id: &ServiceId,
        address: &Address,
    ) -> Result<Vec<(TokenBalanceFromDb, NetworkTokenAddressData)>, ServiceError> {
        let account = repack_address(&address.0)?;
        let balances = self
            .sqlx_client
            .get_token_balances(
                *service_id,
                account.workchain_id(),
                account.address().to_hex_string(),
            )
            .await?;

        let mut result = vec![];
        for balance in balances {
            let root_address = repack_address(&balance.root_address)?;

            let network = self
                .ton_api_client
                .get_token_address_info(&account, &root_address)
                .await?;

            if network.account_status == AccountStatus::UnInit {
                return Err(ServiceError::Other(
                    TonServiceError::AccountNotExist(format!(
                        "{}:{}",
                        network.workchain_id, network.hex
                    ))
                    .into(),
                ));
            }

            result.push((balance, network));
        }
        Ok(result)
    }

    async fn create_send_token_transaction(
        self: Arc<Self>,
        service_id: &ServiceId,
        input: &TokenTransactionSend,
    ) -> Result<TransactionDb, ServiceError> {
        self.request_count
            .send_token_transaction
            .fetch_add(1, Ordering::Relaxed);

        let (_, scale) = input.value.as_bigint_and_exponent();
        if scale != 0 {
            return Err(ServiceError::WrongInput("Invalid token value".to_string()));
        }

        let owner = repack_address(&input.from_address.0)?;
        let address_db = self
            .sqlx_client
            .get_address(
                *service_id,
                owner.workchain_id(),
                owner.address().to_hex_string(),
            )
            .await?;

        if address_db.balance < input.fee {
            return Err(ServiceError::WrongInput(format!(
                "Address balance is not enough to pay fee for token transfer. Balance: {}. Fee: {}",
                address_db.balance, input.fee
            )));
        }

        let token_balance = self
            .sqlx_client
            .get_token_balance(
                *service_id,
                owner.workchain_id(),
                owner.address().to_hex_string(),
                input.root_address.0.clone(),
            )
            .await
            .map_err(|_| {
                ServiceError::WrongInput("Token wallet is not deployed yet".to_string())
            })?;

        if token_balance.balance < input.value {
            return Err(ServiceError::WrongInput(format!(
                "Token balance is not enough to make request; Balance: {}. Sent amount: {}",
                token_balance.balance, input.value
            )));
        }

        let public_key = hex::decode(address_db.public_key.clone())
            .map_err(|err| ServiceError::Other(err.into()))?;
        let private_key = decrypt_private_key(
            &address_db.private_key,
            self.key.as_slice().try_into().trust_me(),
            &address_db.id,
        )
        .map_err(|err| {
            ServiceError::Other(TonServiceError::DecryptPrivateKeyError(err.to_string()).into())
        })?;

        let owner_network = self.ton_api_client.get_address_info(&owner).await?;

        if owner_network.account_status == AccountStatus::UnInit {
            self.deploy_wallet(service_id, &address_db, &public_key, &private_key)
                .await?;
        }

        let (payload, signed_message) = self
            .ton_api_client
            .prepare_token_transaction(
                input,
                &public_key,
                &private_key,
                &address_db.account_type,
                &address_db.custodians,
            )
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_send_transaction(CreateSendTransaction::new(payload, *service_id))
            .await?;

        self.send_transaction(
            transaction.message_hash.clone(),
            transaction.account_hex.clone(),
            transaction.account_workchain_id,
            signed_message,
            true,
        )
        .await
        .trust_me();

        self.notify(service_id, event.into(), NotifyType::Transaction)
            .await?;

        Ok(transaction)
    }

    async fn create_burn_token_transaction(
        self: Arc<Self>,
        service_id: &ServiceId,
        input: &TokenTransactionBurn,
    ) -> Result<TransactionDb, ServiceError> {
        self.request_count
            .send_token_transaction
            .fetch_add(1, Ordering::Relaxed);

        let (_, scale) = input.value.as_bigint_and_exponent();
        if scale != 0 {
            return Err(ServiceError::WrongInput("Invalid token value".to_string()));
        }

        let owner = repack_address(&input.from_address.0)?;
        let address_db = self
            .sqlx_client
            .get_address(
                *service_id,
                owner.workchain_id(),
                owner.address().to_hex_string(),
            )
            .await?;

        if address_db.balance < input.fee {
            return Err(ServiceError::WrongInput(format!(
                "Address balance is not enough to pay fee for token transfer. Balance: {}. Fee: {}",
                address_db.balance, input.fee
            )));
        }

        let token_balance = self
            .sqlx_client
            .get_token_balance(
                *service_id,
                owner.workchain_id(),
                owner.address().to_hex_string(),
                input.root_address.0.clone(),
            )
            .await
            .map_err(|_| {
                ServiceError::WrongInput("Token wallet is not deployed yet".to_string())
            })?;

        if token_balance.balance < input.value {
            return Err(ServiceError::WrongInput(format!(
                "Token balance is not enough to make request; Balance: {}. Sent amount: {}",
                token_balance.balance, input.value
            )));
        }

        let public_key = hex::decode(address_db.public_key.clone())
            .map_err(|err| ServiceError::Other(err.into()))?;
        let private_key = decrypt_private_key(
            &address_db.private_key,
            self.key.as_slice().try_into().trust_me(),
            &address_db.id,
        )
        .map_err(|err| {
            ServiceError::Other(TonServiceError::DecryptPrivateKeyError(err.to_string()).into())
        })?;

        let owner_network = self.ton_api_client.get_address_info(&owner).await?;

        if owner_network.account_status == AccountStatus::UnInit {
            self.deploy_wallet(service_id, &address_db, &public_key, &private_key)
                .await?;
        }

        let (payload, signed_message) = self
            .ton_api_client
            .prepare_token_burn(
                input,
                &public_key,
                &private_key,
                &address_db.account_type,
                &address_db.custodians,
            )
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_send_transaction(CreateSendTransaction::new(payload, *service_id))
            .await?;

        self.send_transaction(
            transaction.message_hash.clone(),
            transaction.account_hex.clone(),
            transaction.account_workchain_id,
            signed_message,
            true,
        )
        .await
        .trust_me();

        self.notify(service_id, event.into(), NotifyType::Transaction)
            .await?;

        Ok(transaction)
    }

    async fn create_mint_token_transaction(
        self: Arc<Self>,
        service_id: &ServiceId,
        input: &TokenTransactionMint,
    ) -> Result<TransactionDb, ServiceError> {
        self.request_count
            .send_token_transaction
            .fetch_add(1, Ordering::Relaxed);

        let (_, scale) = input.value.as_bigint_and_exponent();
        if scale != 0 {
            return Err(ServiceError::WrongInput("Invalid token value".to_string()));
        }

        let (_, scale) = input.deploy_wallet_value.as_bigint_and_exponent();
        if scale != 0 {
            return Err(ServiceError::WrongInput(
                "Invalid deploy wallet value".to_string(),
            ));
        }

        let owner = repack_address(&input.owner_address.0)?;
        let address_db = self
            .sqlx_client
            .get_address(
                *service_id,
                owner.workchain_id(),
                owner.address().to_hex_string(),
            )
            .await?;

        if address_db.balance < input.fee {
            return Err(ServiceError::WrongInput(format!(
                "Address balance is not enough to pay fee for token transfer. Balance: {}. Fee: {}",
                address_db.balance, input.fee
            )));
        }

        let public_key = hex::decode(address_db.public_key.clone())
            .map_err(|err| ServiceError::Other(err.into()))?;
        let private_key = decrypt_private_key(
            &address_db.private_key,
            self.key.as_slice().try_into().trust_me(),
            &address_db.id,
        )
        .map_err(|err| {
            ServiceError::Other(TonServiceError::DecryptPrivateKeyError(err.to_string()).into())
        })?;

        let (payload, signed_message) = self
            .ton_api_client
            .prepare_token_mint(
                input,
                &public_key,
                &private_key,
                &address_db.account_type,
                &address_db.custodians,
            )
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_send_transaction(CreateSendTransaction::new(payload, *service_id))
            .await?;

        self.send_transaction(
            transaction.message_hash.clone(),
            transaction.account_hex.clone(),
            transaction.account_workchain_id,
            signed_message,
            true,
        )
        .await
        .trust_me();

        self.notify(service_id, event.into(), NotifyType::Transaction)
            .await?;

        Ok(transaction)
    }

    async fn create_receive_token_transaction(
        &self,
        input: CreateTokenTransaction,
    ) -> Result<TokenTransactionFromDb, ServiceError> {
        self.request_count
            .recv_token_transaction
            .fetch_add(1, Ordering::Relaxed);

        let address = self
            .sqlx_client
            .get_address_by_workchain_hex(input.account_workchain_id, input.account_hex.clone())
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_token_transaction(input, address.service_id)
            .await?;

        self.notify(
            &address.service_id,
            event.into(),
            NotifyType::TokenTransaction,
        )
        .await?;

        Ok(transaction)
    }

    async fn get_metrics(&self) -> Result<Metrics, ServiceError> {
        Ok(self.ton_api_client.get_metrics().await?)
    }
}

enum NotifyType {
    Transaction,
    TokenTransaction,
}

#[derive(Default)]
struct RequestCount {
    create_address: AtomicU64,
    send_transaction: AtomicU64,
    recv_transaction: AtomicU64,
    send_token_transaction: AtomicU64,
    recv_token_transaction: AtomicU64,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ClientServiceMetrics {
    pub create_address_count: u64,
    pub send_transaction_count: u64,
    pub recv_transaction_count: u64,
    pub send_token_transaction_count: u64,
    pub recv_token_transaction_count: u64,
}

#[derive(thiserror::Error, Debug)]
enum TonServiceError {
    #[error("Account not exist: {0}")]
    AccountNotExist(String),
    #[error("Failed to encrypt private key: {0}")]
    EncryptPrivateKeyError(String),
    #[error("Failed to decrypt private key: {0}")]
    DecryptPrivateKeyError(String),
}
