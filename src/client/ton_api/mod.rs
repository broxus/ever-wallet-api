mod responses;

pub use self::responses::*;

use async_trait::async_trait;
use bigdecimal::BigDecimal;
use ton_block::MsgAddressInt;

use crate::models::address::CreateAddress;
use crate::models::token_transactions::TokenTransactionSend;
use crate::models::transactions::TransactionSend;
use crate::prelude::ServiceError;

#[async_trait]
pub trait TonApiClient: Send + Sync {
    async fn get_address(&self, payload: &CreateAddress) -> Result<CreatedAddress, ServiceError>;
    async fn get_balance(&self, address: MsgAddressInt) -> Result<BigDecimal, ServiceError>;
    async fn get_token_balance(
        &self,
        address: MsgAddressInt,
        root_address: String,
    ) -> Result<BigDecimal, ServiceError>;
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
}

#[derive(Clone, derive_more::Constructor)]
pub struct TonApiClientImpl;

#[async_trait]
impl TonApiClient for TonApiClientImpl {
    async fn get_address(&self, payload: &CreateAddress) -> Result<CreatedAddress, ServiceError> {
        todo!()
    }
    async fn get_balance(&self, address: MsgAddressInt) -> Result<BigDecimal, ServiceError> {
        todo!()
    }
    async fn get_token_balance(
        &self,
        address: MsgAddressInt,
        root_address: String,
    ) -> Result<BigDecimal, ServiceError> {
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
}
