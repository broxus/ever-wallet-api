use async_trait::async_trait;
use uuid::Uuid;

use crate::models::account_enums::TonEventStatus;
use crate::models::address::{Address, CreateAddress};
use crate::models::owners_cache::OwnersCache;
use crate::models::service_id::ServiceId;
use crate::models::sqlx::{AddressDb, TransactionDb, TransactionEventDb};
use crate::models::transactions::TransactionSend;
use crate::prelude::ServiceError;
use crate::sqlx_client::SqlxClient;

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
}

pub struct TonServiceImpl {
    sqlx_client: SqlxClient,
    owners_cache: OwnersCache,
}

impl TonServiceImpl {
    pub fn new(sqlx_client: SqlxClient, owners_cache: OwnersCache) -> Self {
        Self {
            sqlx_client,
            owners_cache,
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
        todo!()
    }
    async fn check_address(
        &self,
        service_id: &ServiceId,
        address: &Address,
    ) -> Result<bool, ServiceError> {
        todo!()
    }
    async fn get_address_balance(
        &self,
        service_id: &ServiceId,
        address: &Address,
    ) -> Result<AddressDb, ServiceError> {
        todo!()
    }
    async fn create_transaction(
        &self,
        service_id: &ServiceId,
        input: &TransactionSend,
    ) -> Result<TransactionDb, ServiceError> {
        todo!()
    }

    async fn get_transaction_by_mh(
        &self,
        service_id: &ServiceId,
        message_hash: &str,
    ) -> Result<TransactionDb, ServiceError> {
        todo!()
    }
    async fn get_transaction_by_h(
        &self,
        service_id: &ServiceId,
        transaction_hash: &str,
    ) -> Result<TransactionDb, ServiceError> {
        todo!()
    }
    async fn search_events(
        &self,
        service_id: &ServiceId,
        event_status: &TonEventStatus,
    ) -> Result<Vec<TransactionEventDb>, ServiceError> {
        todo!()
    }
    async fn mark_event(
        &self,
        service_id: &ServiceId,
        id: &Uuid,
    ) -> Result<Vec<TransactionEventDb>, ServiceError> {
        todo!()
    }
}
