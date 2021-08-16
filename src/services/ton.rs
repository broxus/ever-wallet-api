use std::str::FromStr;

use async_trait::async_trait;
use ton_block::MsgAddressInt;
use uuid::Uuid;

use crate::client::TonApiClient;
use crate::models::account_enums::TonEventStatus;
use crate::models::address::{Address, CreateAddress, CreateAddressInDb};
use crate::models::owners_cache::OwnersCache;
use crate::models::service_id::ServiceId;
use crate::models::sqlx::{
    AddressDb, TokenBalanceFromDb, TokenTransactionEventDb, TokenTransactionFromDb, TransactionDb,
    TransactionEventDb,
};
use crate::models::token_transactions::{CreateSendTokenTransaction, TokenTransactionSend};
use crate::models::transactions::{CreateSendTransaction, TransactionSend};
use crate::prelude::ServiceError;
use crate::sqlx_client::SqlxClient;
use nekoton_utils::unpack_std_smc_addr;
use std::sync::Arc;

#[async_trait]
pub trait TonService: Send + Sync + 'static {
    async fn create_address(
        &self,
        service_id: &ServiceId,
        input: &CreateAddress,
    ) -> Result<AddressDb, ServiceError>;
    async fn check_address(
        &self,
        service_id: &ServiceId,
        address: &Address,
    ) -> Result<bool, ServiceError>;
    async fn get_address_balance(
        &self,
        service_id: &ServiceId,
        address: &Address,
    ) -> Result<AddressDb, ServiceError>;
    async fn create_transaction(
        &self,
        service_id: &ServiceId,
        input: &TransactionSend,
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
    async fn search_events(
        &self,
        service_id: &ServiceId,
        event_status: &TonEventStatus,
    ) -> Result<Vec<TransactionEventDb>, ServiceError>;
    async fn mark_event(
        &self,
        service_id: &ServiceId,
        id: &Uuid,
    ) -> Result<Vec<TransactionEventDb>, ServiceError>;
    async fn get_tokens_transaction_by_mh(
        &self,
        service_id: &ServiceId,
        message_hash: &str,
    ) -> Result<TokenTransactionFromDb, ServiceError>;
    async fn search_token_events(
        &self,
        service_id: &ServiceId,
        event_status: &TonEventStatus,
    ) -> Result<Vec<TokenTransactionEventDb>, ServiceError>;
    async fn mark_token_event(
        &self,
        service_id: &ServiceId,
        id: &Uuid,
    ) -> Result<Vec<TokenTransactionEventDb>, ServiceError>;
    async fn get_token_address_balance(
        &self,
        service_id: &ServiceId,
        address: &Address,
    ) -> Result<Vec<TokenBalanceFromDb>, ServiceError>;
    async fn create_token_transaction(
        &self,
        service_id: &ServiceId,
        input: &TokenTransactionSend,
    ) -> Result<TokenTransactionFromDb, ServiceError>;
}

pub struct TonServiceImpl {
    sqlx_client: SqlxClient,
    owners_cache: OwnersCache,
    ton_api_client: Arc<dyn TonApiClient>,
}

impl TonServiceImpl {
    pub fn new(
        sqlx_client: SqlxClient,
        owners_cache: OwnersCache,
        ton_api_client: Arc<dyn TonApiClient>,
    ) -> Self {
        Self {
            sqlx_client,
            owners_cache,
            ton_api_client,
        }
    }
}

#[async_trait]
impl TonService for TonServiceImpl {
    async fn create_address(
        &self,
        service_id: &ServiceId,
        input: &CreateAddress,
    ) -> Result<AddressDb, ServiceError> {
        let payload = self.ton_api_client.get_address(input).await?;
        self.sqlx_client
            .create_address(CreateAddressInDb::new(payload, *service_id))
            .await
    }
    async fn check_address(
        &self,
        service_id: &ServiceId,
        address: &Address,
    ) -> Result<bool, ServiceError> {
        Ok(MsgAddressInt::from_str(&address.0).is_ok()
            || (unpack_std_smc_addr(&address.0, true).is_ok()))
    }
    async fn get_address_balance(
        &self,
        service_id: &ServiceId,
        address: &Address,
    ) -> Result<AddressDb, ServiceError> {
        let account = MsgAddressInt::from_str(&address.0).map_err(|_| {
            ServiceError::WrongInput(format!("Can not parse Address workchain and hex"))
        })?;
        self.sqlx_client
            .get_address(
                *service_id,
                account.workchain_id(),
                account.address().to_hex_string(),
            )
            .await
    }
    async fn create_transaction(
        &self,
        service_id: &ServiceId,
        input: &TransactionSend,
    ) -> Result<TransactionDb, ServiceError> {
        let payload = self.ton_api_client.prepare_transaction(input).await?;
        let result = self
            .sqlx_client
            .create_send_transaction(CreateSendTransaction::new(payload.clone(), *service_id))
            .await?;
        self.ton_api_client.send_transaction(&payload).await?;
        Ok(result)
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
    async fn search_events(
        &self,
        service_id: &ServiceId,
        event_status: &TonEventStatus,
    ) -> Result<Vec<TransactionEventDb>, ServiceError> {
        self.sqlx_client
            .get_transaction_events(*service_id, *event_status)
            .await
    }
    async fn mark_event(
        &self,
        service_id: &ServiceId,
        id: &Uuid,
    ) -> Result<Vec<TransactionEventDb>, ServiceError> {
        todo!()
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
    async fn search_token_events(
        &self,
        service_id: &ServiceId,
        event_status: &TonEventStatus,
    ) -> Result<Vec<TokenTransactionEventDb>, ServiceError> {
        self.sqlx_client
            .get_token_transaction_events(*service_id, *event_status)
            .await
    }
    async fn mark_token_event(
        &self,
        service_id: &ServiceId,
        id: &Uuid,
    ) -> Result<Vec<TokenTransactionEventDb>, ServiceError> {
        todo!()
    }
    async fn get_token_address_balance(
        &self,
        service_id: &ServiceId,
        address: &Address,
    ) -> Result<Vec<TokenBalanceFromDb>, ServiceError> {
        let account = MsgAddressInt::from_str(&address.0).map_err(|_| {
            ServiceError::WrongInput(format!("Can not parse address workchain and hex"))
        })?;
        self.sqlx_client
            .get_token_balances(
                *service_id,
                account.workchain_id(),
                account.address().to_hex_string(),
            )
            .await
    }
    async fn create_token_transaction(
        &self,
        service_id: &ServiceId,
        input: &TokenTransactionSend,
    ) -> Result<TokenTransactionFromDb, ServiceError> {
        let payload = self.ton_api_client.prepare_token_transaction(input).await?;
        let result = self
            .sqlx_client
            .create_send_token_transaction(CreateSendTokenTransaction::new(
                payload.clone(),
                *service_id,
            ))
            .await?;
        self.ton_api_client.send_token_transaction(&payload).await?;
        Ok(result)
    }
}
