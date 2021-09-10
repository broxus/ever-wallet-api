use std::str::FromStr;
use std::sync::Arc;

use aes::Aes256;
use async_trait::async_trait;
use bigdecimal::BigDecimal;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Ecb};
use nekoton::core::models::TokenWalletVersion;
use nekoton::crypto::SignedMessage;
use nekoton_utils::{repack_address, unpack_std_smc_addr, TrustMe};
use sha2::{Digest, Sha256};
use ton_block::MsgAddressInt;
use ton_types::UInt256;
use uuid::Uuid;

use crate::client::*;
use crate::models::*;
use crate::prelude::*;
use crate::sqlx_client::*;
use crate::utils::*;

type Aes128Ecb = Ecb<Aes256, Pkcs7>;

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
    async fn create_send_transaction(
        &self,
        service_id: &ServiceId,
        input: TransactionSend,
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
    async fn get_metrics(&self) -> Result<Metrics, ServiceError>;
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
        &self,
        service_id: &ServiceId,
        input: &TokenTransactionSend,
    ) -> Result<TransactionDb, ServiceError>;
    async fn create_token_transaction(
        &self,
        input: CreateTokenTransaction,
    ) -> Result<TokenTransactionFromDb, ServiceError>;
}

#[derive(Clone)]
pub struct TonServiceImpl {
    sqlx_client: SqlxClient,
    owners_cache: OwnersCache,
    ton_api_client: Arc<dyn TonClient>,
    callback_client: Arc<dyn CallbackClient>,
    secret: String,
}

impl TonServiceImpl {
    pub fn new(
        sqlx_client: SqlxClient,
        owners_cache: OwnersCache,
        ton_api_client: Arc<dyn TonClient>,
        callback_client: Arc<dyn CallbackClient>,
        secret: String,
    ) -> Self {
        Self {
            sqlx_client,
            owners_cache,
            ton_api_client,
            callback_client,
            secret,
        }
    }
    async fn notify_token(&self, service_id: &ServiceId, payload: AccountTransactionEvent) {
        if let Ok(url) = self.sqlx_client.get_callback(*service_id).await {
            let secret = self
                .sqlx_client
                .get_key_by_service_id(service_id)
                .await
                .map(|k| k.secret)
                .unwrap_or_default();
            let event_status = match self
                .callback_client
                .send(url, payload.clone(), secret)
                .await
            {
                Err(e) => {
                    log::error!("{}", e);
                    TonEventStatus::Error
                }
                Ok(_) => TonEventStatus::Notified,
            };
            if let Err(e) = self
                .sqlx_client
                .update_event_status_of_token_transaction_event(
                    payload.message_hash,
                    payload.account.workchain_id,
                    payload.account.hex.0,
                    event_status,
                )
                .await
            {
                log::error!("{}", e);
            }
        }
    }
    async fn notify(&self, service_id: &ServiceId, payload: AccountTransactionEvent) {
        if let Ok(url) = self.sqlx_client.get_callback(*service_id).await {
            let secret = self
                .sqlx_client
                .get_key_by_service_id(service_id)
                .await
                .map(|k| k.secret)
                .unwrap_or_default();
            let event_status = match self
                .callback_client
                .send(url, payload.clone(), secret)
                .await
            {
                Err(e) => {
                    log::error!("{}", e);
                    TonEventStatus::Error
                }
                Ok(_) => TonEventStatus::Notified,
            };
            if let Err(e) = self
                .sqlx_client
                .update_event_status_of_transaction_event(
                    payload.message_hash,
                    payload.account.workchain_id,
                    payload.account.hex.0,
                    event_status,
                )
                .await
            {
                log::error!("{}", e);
            }
        }
    }

    async fn encrypt_private_key(&self, private_key: &[u8]) -> String {
        // create sha256 from secret
        let mut hasher = Sha256::new();
        hasher.update(&self.secret);
        let secret_sha256 = hasher.finalize();

        // encrypt address private key
        let cipher = Aes128Ecb::new_from_slices(&secret_sha256, &secret_sha256).unwrap();
        let mut buffer = [0u8; 64];
        let pos = private_key.len();
        buffer[..pos].copy_from_slice(private_key);
        let ciphertext = cipher.encrypt(&mut buffer, pos).unwrap();
        base64::encode(ciphertext)
    }

    async fn decrypt_private_key(&self, private_key: String) -> Vec<u8> {
        // create sha256 from secret
        let mut hasher = Sha256::new();
        hasher.update(&self.secret);
        let secret_sha256 = hasher.finalize();

        // decrypt address private key
        let private_key = base64::decode(private_key).unwrap_or_default();
        let cipher = Aes128Ecb::new_from_slices(&secret_sha256, &secret_sha256).unwrap();
        let mut buf = private_key.to_vec();
        cipher.decrypt(&mut buf).unwrap().to_vec()
    }

    async fn send_transaction(
        &self,
        message_hash: String,
        account_hex: String,
        account_workchain_id: i32,
        signed_message: SignedMessage,
        non_blocking: bool,
    ) -> Result<(), ServiceError> {
        if non_blocking {
            let ton_service = self.clone();
            tokio::spawn(async move {
                if let Err(err) = send_transaction_helper(
                    &ton_service,
                    message_hash,
                    account_hex,
                    account_workchain_id,
                    signed_message,
                )
                .await
                {
                    log::error!("{:?}", err);
                };
            });
            Ok(())
        } else {
            send_transaction_helper(
                self,
                message_hash,
                account_hex,
                account_workchain_id,
                signed_message,
            )
            .await
        }
    }
}

#[async_trait]
impl TonService for TonServiceImpl {
    async fn create_address(
        &self,
        service_id: &ServiceId,
        input: CreateAddress,
    ) -> Result<AddressDb, ServiceError> {
        let payload = self.ton_api_client.create_address(input).await?;

        let public_key = hex::encode(&payload.public_key);
        let private_key = self.encrypt_private_key(&payload.private_key).await;

        self.sqlx_client
            .create_address(CreateAddressInDb::new(
                payload,
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
        let account = repack_address(&address.0)?;
        let address = self
            .sqlx_client
            .get_address(
                *service_id,
                account.workchain_id(),
                account.address().to_hex_string(),
            )
            .await?;
        let network = self.ton_api_client.get_address_info(account).await?;
        Ok((address, network))
    }

    async fn create_send_transaction(
        &self,
        service_id: &ServiceId,
        input: TransactionSend,
    ) -> Result<TransactionDb, ServiceError> {
        let account = repack_address(&input.from_address.0)?;

        let network = self
            .ton_api_client
            .get_address_info(account.clone())
            .await?;

        if input
            .outputs
            .iter()
            .map(|o| o.value.clone())
            .sum::<BigDecimal>()
            >= network.network_balance
        {
            return Err(ServiceError::WrongInput("Insufficient balance".to_string()));
        }

        let address = self
            .sqlx_client
            .get_address(
                *service_id,
                account.workchain_id(),
                account.address().to_hex_string(),
            )
            .await?;

        let public_key = hex::decode(address.public_key.clone()).unwrap_or_default();
        let secret = self.decrypt_private_key(address.private_key.clone()).await;

        if network.account_status == AccountStatus::UnInit {
            let payload = self
                .ton_api_client
                .prepare_deploy(&address, &public_key, &secret)
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

                self.notify(service_id, event.into()).await;
            }
        }

        let (payload, signed_message) = self
            .ton_api_client
            .prepare_transaction(
                input,
                &public_key,
                &secret,
                &address.account_type,
                &address.custodians,
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

        self.notify(service_id, event.into()).await;

        Ok(transaction)
    }

    async fn create_receive_transaction(
        &self,
        input: CreateReceiveTransaction,
    ) -> Result<TransactionDb, ServiceError> {
        let address = self
            .sqlx_client
            .get_address_by_workchain_hex(input.account_workchain_id, input.account_hex.clone())
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_receive_transaction(input, address.service_id)
            .await?;

        self.notify(&address.service_id, event.into()).await;

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

        let (transaction, event) = if address.account_type == AccountType::SafeMultisig
            && address.custodians.unwrap_or_default() > 1
        {
            if self
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
            }
        } else {
            self.sqlx_client
                .update_send_transaction(message_hash, account_workchain_id, account_hex, input)
                .await?
        };

        self.notify(&address.service_id, event.into()).await;

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

    async fn get_metrics(&self) -> Result<Metrics, ServiceError> {
        Ok(self.ton_api_client.get_metrics().await?)
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
            result.push((balance, network));
        }
        Ok(result)
    }

    async fn create_send_token_transaction(
        &self,
        service_id: &ServiceId,
        input: &TokenTransactionSend,
    ) -> Result<TransactionDb, ServiceError> {
        let owner = repack_address(&input.from_address.0)?;
        let address = self
            .sqlx_client
            .get_address(
                *service_id,
                owner.workchain_id(),
                owner.address().to_hex_string(),
            )
            .await?;

        if address.balance < input.fee {
            return Err(ServiceError::WrongInput(format!(
                "Address balance is not enough to pay fee for token transfer. Balance: {}. Fee: {}",
                address.balance, input.fee
            )));
        }

        let root_address = repack_address(&input.root_address)?;

        let owner_info = self
            .ton_api_client
            .get_token_address_info(&owner, &root_address)
            .await?;

        if owner_info.account_status == AccountStatus::UnInit {
            return Err(ServiceError::WrongInput(format!(
                "Token wallet not found for `{}`",
                owner.to_string()
            )));
        }
        let token_wallet =
            MsgAddressInt::from_str(&format!("{}:{}", owner_info.workchain_id, owner_info.hex))
                .unwrap_or_default();

        let recipient = repack_address(&input.recipient_address.0)?;
        let recipient_info = self
            .ton_api_client
            .get_token_address_info(&recipient, &root_address)
            .await?;
        let recipient_token_wallet = MsgAddressInt::from_str(&format!(
            "{}:{}",
            recipient_info.workchain_id, recipient_info.hex
        ))
        .unwrap_or_default();

        let destination = match recipient_info.account_status {
            AccountStatus::Active => {
                nekoton::core::models::TransferRecipient::TokenWallet(recipient_token_wallet)
            }
            AccountStatus::UnInit => {
                nekoton::core::models::TransferRecipient::OwnerWallet(recipient)
            }
            AccountStatus::Frozen => {
                return Err(ServiceError::WrongInput(format!(
                    "Destination token wallet `{}` is frozen",
                    recipient.to_string()
                )));
            }
        };

        let tokens = input.value.clone();
        let version = TokenWalletVersion::from_str(&owner_info.version)?;

        let public_key = hex::decode(address.public_key).unwrap_or_default();
        let private_key = self.decrypt_private_key(address.private_key).await;

        let (payload, signed_message) = self
            .ton_api_client
            .prepare_token_transaction(
                input.id,
                owner,
                token_wallet,
                destination,
                version,
                tokens,
                address.account_type,
                &public_key,
                &private_key,
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

        self.notify_token(service_id, event.into()).await;

        Ok(transaction)
    }

    async fn create_token_transaction(
        &self,
        input: CreateTokenTransaction,
    ) -> Result<TokenTransactionFromDb, ServiceError> {
        let address = self
            .sqlx_client
            .get_address_by_workchain_hex(input.account_workchain_id, input.account_hex.clone())
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_token_transaction(input, address.service_id)
            .await?;

        self.notify_token(&address.service_id, event.into()).await;

        Ok(transaction)
    }
}

async fn send_transaction_helper(
    ton_service: &TonServiceImpl,
    message_hash: String,
    account_hex: String,
    account_workchain_id: i32,
    signed_message: SignedMessage,
) -> Result<(), ServiceError> {
    let account = UInt256::from_be_bytes(&hex::decode(&account_hex).unwrap_or_default());
    match ton_service
        .ton_api_client
        .send_transaction(account, signed_message)
        .await
    {
        Ok(MessageStatus::Delivered) => {
            log::info!("Successfully sent message `{}`", message_hash);
            Ok(())
        }
        Ok(MessageStatus::Expired) => {
            log::info!("Message `{}` expired", message_hash);
            match ton_service
                .upsert_sent_transaction(
                    message_hash,
                    account_workchain_id,
                    account_hex,
                    UpdateSendTransaction::error("Expired".to_string()),
                )
                .await
            {
                Ok(_) => Ok(()),
                Err(err) => Err(ServiceError::Other(
                    TonServiceError::UpdateMessageFail(err).into(),
                )),
            }
        }
        Err(e) => {
            log::error!("Failed to send message: {:?}", e);
            match ton_service
                .upsert_sent_transaction(
                    message_hash,
                    account_workchain_id,
                    account_hex,
                    UpdateSendTransaction::error("Fail".to_string()),
                )
                .await
            {
                Ok(_) => Ok(()),
                Err(err) => Err(ServiceError::Other(
                    TonServiceError::UpdateMessageFail(err).into(),
                )),
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
enum TonServiceError {
    #[error("Failed to update sent transaction: {0}")]
    UpdateMessageFail(ServiceError),
}
