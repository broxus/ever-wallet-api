use std::str::FromStr;
use std::sync::Arc;

use aes::Aes128;
use async_trait::async_trait;
use block_modes::block_padding::NoPadding;
use block_modes::{BlockMode, Cbc};
use nekoton_utils::unpack_std_smc_addr;
use sha2::{Digest, Sha256};
use ton_block::MsgAddressInt;
use uuid::Uuid;

use crate::client::{AccountTransactionEvent, CallbackClient, TonClient};
use crate::models::account_enums::{AccountStatus, TonEventStatus};
use crate::models::address::{Address, CreateAddress, CreateAddressInDb, NetworkAddressData};
use crate::models::owners_cache::OwnersCache;
use crate::models::service_id::ServiceId;
use crate::models::sqlx::{
    AddressDb, TokenBalanceFromDb, TokenTransactionEventDb, TokenTransactionFromDb, TransactionDb,
    TransactionEventDb,
};
use crate::models::token_balance::NetworkTokenAddressData;
use crate::models::token_transaction_events::TokenTransactionsEventsSearch;
use crate::models::token_transactions::{
    CreateReceiveTokenTransaction, CreateSendTokenTransaction, TokenTransactionSend,
    UpdateSendTokenTransaction,
};
use crate::models::transaction_events::TransactionsEventsSearch;
use crate::models::transactions::{
    CreateReceiveTransaction, CreateSendTransaction, TransactionSend, UpdateSendTransaction,
};
use crate::prelude::ServiceError;
use crate::sqlx_client::SqlxClient;

type Aes128Cbc = Cbc<Aes128, NoPadding>;

#[async_trait]
pub trait TonService: Send + Sync + 'static {
    async fn create_address(
        &self,
        service_id: &ServiceId,
        input: &CreateAddress,
    ) -> Result<AddressDb, ServiceError>;
    async fn check_address(&self, address: &Address) -> Result<bool, ServiceError>;
    async fn get_address_balance(
        &self,
        service_id: &ServiceId,
        address: &Address,
    ) -> Result<(AddressDb, NetworkAddressData), ServiceError>;
    async fn create_send_transaction(
        &self,
        service_id: &ServiceId,
        input: &TransactionSend,
    ) -> Result<TransactionDb, ServiceError>;
    async fn create_receive_transaction(
        &self,
        input: &CreateReceiveTransaction,
    ) -> Result<TransactionDb, ServiceError>;
    async fn update_sent_transaction(
        &self,
        message_hash: String,
        account_workchain_id: i32,
        account_hex: String,
        input: &UpdateSendTransaction,
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
        &self,
        service_id: &ServiceId,
        input: &TokenTransactionSend,
    ) -> Result<TokenTransactionFromDb, ServiceError>;
    async fn create_receive_token_transaction(
        &self,
        input: &CreateReceiveTokenTransaction,
    ) -> Result<TokenTransactionFromDb, ServiceError>;
    async fn update_sent_token_transaction(
        &self,
        message_hash: String,
        account_workchain_id: i32,
        account_hex: String,
        root_address: String,
        input: &UpdateSendTokenTransaction,
    ) -> Result<TokenTransactionFromDb, ServiceError>;
}

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
    async fn notify_token(&self, service_id: ServiceId, payload: AccountTransactionEvent) {
        if let Ok(url) = self.sqlx_client.get_callback(service_id).await {
            let event_status = match self.callback_client.send(url, payload.clone()).await {
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
    async fn notify(&self, service_id: ServiceId, payload: AccountTransactionEvent) {
        if let Ok(url) = self.sqlx_client.get_callback(service_id).await {
            let event_status = match self.callback_client.send(url, payload.clone()).await {
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
        let cipher = Aes128Cbc::new_from_slices(&secret_sha256, &secret_sha256).unwrap();
        let mut buffer = [0u8; 32];
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
        let cipher = Aes128Cbc::new_from_slices(&secret_sha256, &secret_sha256).unwrap();
        let mut buf = private_key.to_vec();
        cipher.decrypt(&mut buf).unwrap().to_vec()
    }
}

#[async_trait]
impl TonService for TonServiceImpl {
    async fn create_address(
        &self,
        service_id: &ServiceId,
        input: &CreateAddress,
    ) -> Result<AddressDb, ServiceError> {
        let payload = self.ton_api_client.create_address(input).await?;
        self.sqlx_client
            .create_address(CreateAddressInDb::new(payload, *service_id))
            .await
    }
    async fn check_address(&self, address: &Address) -> Result<bool, ServiceError> {
        Ok(MsgAddressInt::from_str(&address.0).is_ok()
            || (unpack_std_smc_addr(&address.0, true).is_ok()))
    }
    async fn get_address_balance(
        &self,
        service_id: &ServiceId,
        address: &Address,
    ) -> Result<(AddressDb, NetworkAddressData), ServiceError> {
        let account = MsgAddressInt::from_str(&address.0).map_err(|_| {
            ServiceError::WrongInput(format!("Can not parse Address workchain and hex"))
        })?;
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
        input: &TransactionSend,
    ) -> Result<TransactionDb, ServiceError> {
        let account = MsgAddressInt::from_str(&input.from_address.0).map_err(|_| {
            ServiceError::WrongInput(format!("Can not parse Address workchain and hex"))
        })?;
        let network = self
            .ton_api_client
            .get_address_info(account.clone())
            .await?;
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
            self.ton_api_client
                .deploy_address_contract(&address, &secret)
                .await?;
        }
        let (payload, unsigned_message) = self
            .ton_api_client
            .prepare_transaction(input, &public_key, address.account_type)
            .await?;
        let (mut transaction, mut event) = self
            .sqlx_client
            .create_send_transaction(CreateSendTransaction::new(payload.clone(), *service_id))
            .await?;
        if let Err(e) = self
            .ton_api_client
            .send_transaction(unsigned_message, &public_key, &secret)
            .await
        {
            let result = self
                .sqlx_client
                .update_send_transaction(
                    transaction.message_hash,
                    transaction.account_workchain_id,
                    transaction.account_hex,
                    UpdateSendTransaction::error(e.to_string()),
                )
                .await?;
            transaction = result.0;
            event = result.1;
        }
        self.notify(*service_id, event.into()).await;

        Ok(transaction)
    }

    async fn create_receive_transaction(
        &self,
        input: &CreateReceiveTransaction,
    ) -> Result<TransactionDb, ServiceError> {
        let address = self
            .sqlx_client
            .get_address_by_workchain_hex(input.account_workchain_id, input.account_hex.clone())
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_receive_transaction(input.clone(), address.service_id)
            .await?;

        self.notify(address.service_id, event.into()).await;

        Ok(transaction)
    }

    async fn update_sent_transaction(
        &self,
        message_hash: String,
        account_workchain_id: i32,
        account_hex: String,
        input: &UpdateSendTransaction,
    ) -> Result<TransactionDb, ServiceError> {
        let address = self
            .sqlx_client
            .get_address_by_workchain_hex(account_workchain_id, account_hex.clone())
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .update_send_transaction(
                message_hash,
                account_workchain_id,
                account_hex,
                input.clone(),
            )
            .await?;

        self.notify(address.service_id, event.into()).await;

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
        let account = MsgAddressInt::from_str(&address.0).map_err(|_| {
            ServiceError::WrongInput(format!("Can not parse address workchain and hex"))
        })?;
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
            let root_address = MsgAddressInt::from_str(&balance.root_address).map_err(|_| {
                ServiceError::WrongInput(format!("Can not parse root address workchain and hex"))
            })?;

            let network = self
                .ton_api_client
                .get_token_address_info(account.clone(), root_address)
                .await?;
            result.push((balance, network));
        }
        Ok(result)
    }
    async fn create_send_token_transaction(
        &self,
        service_id: &ServiceId,
        input: &TokenTransactionSend,
    ) -> Result<TokenTransactionFromDb, ServiceError> {
        let account = MsgAddressInt::from_str(&input.from_address.0).map_err(|_| {
            ServiceError::WrongInput(format!("Can not parse Address workchain and hex"))
        })?;
        let address = self
            .sqlx_client
            .get_address(
                *service_id,
                account.workchain_id(),
                account.address().to_hex_string(),
            )
            .await?;
        if address.balance < input.fee {
            return Err(ServiceError::WrongInput(format!(
                "Address balance is not enough to pay fee for token transfer. Balance: {}. Fee: {}",
                address.balance, input.fee
            )));
        }

        let root_address = MsgAddressInt::from_str(&input.root_address).map_err(|_| {
            ServiceError::WrongInput(format!("Can not parse root address workchain and hex"))
        })?;

        let network = self
            .ton_api_client
            .get_token_address_info(account.clone(), root_address.clone())
            .await?;
        if network.account_status == AccountStatus::UnInit {
            let token_balance = self
                .sqlx_client
                .get_token_balance(
                    *service_id,
                    account.workchain_id(),
                    account.address().to_hex_string(),
                    input.root_address.clone(),
                )
                .await?;
            self.ton_api_client
                .deploy_token_address_contract(
                    token_balance,
                    address.public_key.clone(),
                    address.private_key.clone(),
                    address.account_type,
                )
                .await?;
        }

        let payload = self
            .ton_api_client
            .prepare_token_transaction(
                input,
                address.public_key.clone(),
                address.private_key.clone(),
                address.account_type,
            )
            .await?;
        let (mut transaction, mut event) = self
            .sqlx_client
            .create_send_token_transaction(CreateSendTokenTransaction::new(
                payload.clone(),
                *service_id,
            ))
            .await?;
        if let Err(e) = self
            .ton_api_client
            .send_token_transaction(
                &payload,
                address.public_key,
                address.private_key,
                address.account_type,
            )
            .await
        {
            let result = self
                .sqlx_client
                .update_send_token_transaction(
                    transaction.message_hash,
                    transaction.account_workchain_id,
                    transaction.account_hex,
                    transaction.root_address,
                    UpdateSendTokenTransaction::error(e.to_string()),
                )
                .await?;
            transaction = result.0;
            event = result.1;
        }

        self.notify_token(*service_id, event.into()).await;

        Ok(transaction)
    }

    async fn create_receive_token_transaction(
        &self,
        input: &CreateReceiveTokenTransaction,
    ) -> Result<TokenTransactionFromDb, ServiceError> {
        let address = self
            .sqlx_client
            .get_token_balance_by_workchain_hex(
                input.account_workchain_id,
                input.account_hex.clone(),
                input.root_address.clone(),
            )
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_receive_token_transaction(input.clone(), address.service_id)
            .await?;

        self.notify_token(address.service_id, event.into()).await;

        Ok(transaction)
    }

    async fn update_sent_token_transaction(
        &self,
        message_hash: String,
        account_workchain_id: i32,
        account_hex: String,
        root_address: String,
        input: &UpdateSendTokenTransaction,
    ) -> Result<TokenTransactionFromDb, ServiceError> {
        let address = self
            .sqlx_client
            .get_token_balance_by_workchain_hex(
                account_workchain_id,
                account_hex.clone(),
                root_address.clone(),
            )
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .update_send_token_transaction(
                message_hash,
                account_workchain_id,
                account_hex,
                root_address,
                input.clone(),
            )
            .await?;

        self.notify_token(address.service_id, event.into()).await;

        Ok(transaction)
    }
}
