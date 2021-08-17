mod responses;

pub use self::responses::*;

use async_trait::async_trait;
use ton_block::MsgAddressInt;

use crate::models::address::{CreateAddress, NetworkAddressData};
use crate::models::sqlx::{AddressDb, TokenBalanceFromDb};
use crate::models::token_balance::NetworkTokenAddressData;
use crate::models::token_transactions::TokenTransactionSend;
use crate::models::transactions::TransactionSend;
use crate::prelude::ServiceError;

#[async_trait]
pub trait TonClient: Send + Sync {
    async fn create_address(&self, payload: &CreateAddress)
        -> Result<CreatedAddress, ServiceError>;
    async fn get_address_info(
        &self,
        address: MsgAddressInt,
    ) -> Result<NetworkAddressData, ServiceError>;
    async fn deploy_address_contract(&self, address: AddressDb) -> Result<(), ServiceError>;
    async fn get_token_address_info(
        &self,
        address: MsgAddressInt,
        root_address: String,
    ) -> Result<NetworkTokenAddressData, ServiceError>;
    async fn prepare_transaction(
        &self,
        transaction: &TransactionSend,
    ) -> Result<SentTransaction, ServiceError>;
    async fn send_transaction(&self, transaction: &SentTransaction) -> Result<(), ServiceError>;
    async fn prepare_token_transaction(
        &self,
        transaction: &TokenTransactionSend,
    ) -> Result<SentTokenTransaction, ServiceError>;
    async fn send_token_transaction(
        &self,
        transaction: &SentTokenTransaction,
    ) -> Result<(), ServiceError>;
    async fn deploy_token_address_contract(
        &self,
        address: TokenBalanceFromDb,
    ) -> Result<(), ServiceError>;
}

#[derive(Clone, derive_more::Constructor)]
pub struct TonClientImpl;

#[async_trait]
impl TonClient for TonClientImpl {
    async fn create_address(
        &self,
        payload: &CreateAddress,
    ) -> Result<CreatedAddress, ServiceError> {
        todo!()
    }
    async fn get_address_info(
        &self,
        address: MsgAddressInt,
    ) -> Result<NetworkAddressData, ServiceError> {
        todo!()
    }
    async fn deploy_address_contract(&self, address: AddressDb) -> Result<(), ServiceError> {
        todo!()
    }
    async fn get_token_address_info(
        &self,
        address: MsgAddressInt,
        root_address: String,
    ) -> Result<NetworkTokenAddressData, ServiceError> {
        todo!()
    }
    async fn prepare_transaction(
        &self,
        transaction: &TransactionSend,
    ) -> Result<SentTransaction, ServiceError> {
        todo!()
    }
    async fn send_transaction(&self, transaction: &SentTransaction) -> Result<(), ServiceError> {
        todo!()
    }
    async fn prepare_token_transaction(
        &self,
        transaction: &TokenTransactionSend,
    ) -> Result<SentTokenTransaction, ServiceError> {
        todo!()
    }
    async fn send_token_transaction(
        &self,
        transaction: &SentTokenTransaction,
    ) -> Result<(), ServiceError> {
        todo!()
    }
    async fn deploy_token_address_contract(
        &self,
        address: TokenBalanceFromDb,
    ) -> Result<(), ServiceError> {
        todo!()
    }
}
