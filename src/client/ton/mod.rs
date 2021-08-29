use std::convert::TryFrom;
use std::sync::Arc;

use async_trait::async_trait;
use bigdecimal::BigDecimal;
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer};
use nekoton_utils::TrustMe;
use num_traits::FromPrimitive;
use ton_block::{AccountState, MsgAddressInt};
use ton_types::UInt256;

use crate::models::account_enums::{AccountStatus, AccountType};
use crate::models::address::{CreateAddress, NetworkAddressData};
use crate::models::sqlx::{AddressDb, TokenBalanceFromDb};
use crate::models::token_balance::NetworkTokenAddressData;
use crate::models::token_transactions::TokenTransactionSend;
use crate::models::transactions::TransactionSend;
use crate::prelude::ServiceError;
use crate::ton_core::TonCore;

pub use self::responses::*;
pub use self::token_wallet::*;
pub use self::ton_wallet::*;

mod responses;
mod token_wallet;
mod ton_wallet;

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
        public_key: String,
        private_key: String,
        account_type: AccountType,
    ) -> Result<SentTransaction, ServiceError>;
    async fn send_transaction(
        &self,
        transaction: &SentTransaction,
        public_key: String,
        private_key: String,
        account_type: AccountType,
    ) -> Result<(), ServiceError>;
    async fn prepare_token_transaction(
        &self,
        transaction: &TokenTransactionSend,
        public_key: String,
        private_key: String,
        account_type: AccountType,
    ) -> Result<SentTokenTransaction, ServiceError>;
    async fn send_token_transaction(
        &self,
        transaction: &SentTokenTransaction,
        public_key: String,
        private_key: String,
        account_type: AccountType,
    ) -> Result<(), ServiceError>;
    async fn deploy_token_address_contract(
        &self,
        address: TokenBalanceFromDb,
        public_key: String,
        private_key: String,
        account_type: AccountType,
    ) -> Result<(), ServiceError>;
}

#[derive(Clone)]
pub struct TonClientImpl {
    ton_core: Arc<TonCore>,
}

impl TonClientImpl {
    pub fn new(ton_core: Arc<TonCore>) -> Self {
        Self { ton_core }
    }
}

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
        let account = UInt256::from_be_bytes(&address.address().get_bytestring(0));
        let addr_info = self.ton_core.get_ton_address_info(account).await?;

        let workchain_id = addr_info.workchain_id;
        let hex = addr_info.hex;

        let account_status = match addr_info.account_status {
            AccountState::AccountUninit => AccountStatus::UnInit,
            AccountState::AccountActive(_) => AccountStatus::Active,
            AccountState::AccountFrozen(_) => AccountStatus::Frozen,
        };

        let network_balance =
            BigDecimal::from_u128(addr_info.network_balance).ok_or_else(|| {
                ServiceError::NotFound(format!("Failed to get balance for {}", address))
            })?;
        let last_transaction_hash = addr_info
            .last_transaction_hash
            .map(|hash| hash.to_hex_string());
        let last_transaction_lt = addr_info.last_transaction_lt.map(|lt| lt.to_string());

        Ok(NetworkAddressData {
            workchain_id,
            hex,
            account_status,
            network_balance,
            last_transaction_hash,
            last_transaction_lt,
            sync_u_time: 0, // TODO
        })
    }
    async fn deploy_address_contract(&self, address: AddressDb) -> Result<(), ServiceError> {
        let data = PrepareDeploy::try_from(address)?;
        let unsigned_message = match data.account_type {
            AccountType::HighloadWallet => {
                nekoton::core::ton_wallet::highload_wallet_v2::prepare_deploy(
                    &data.public_key,
                    data.workchain,
                    data.expiration,
                )?
            }
            AccountType::Wallet => nekoton::core::ton_wallet::wallet_v3::prepare_deploy(
                &data.public_key,
                data.workchain,
                data.expiration,
            )?,
            AccountType::SafeMultisig => nekoton::core::ton_wallet::multisig::prepare_deploy(
                &data.public_key,
                MULTISIG_TYPE,
                data.workchain,
                data.expiration,
                &data.owners.trust_me(),
                data.req_confirms.trust_me(),
            )?,
        };

        let key_pair = Keypair {
            secret: data.secret,
            public: data.public_key,
        };

        let signature = key_pair.sign(unsigned_message.hash());
        let signed_message = unsigned_message.sign(&signature.to_bytes())?;

        self.ton_core
            .send_ton_message(&signed_message.message, signed_message.expire_at)
            .await
            .map_err(ServiceError::Other)
    }
    async fn prepare_transaction(
        &self,
        transaction: &TransactionSend,
        public_key: String,
        private_key: String,
        account_type: AccountType,
    ) -> Result<SentTransaction, ServiceError> {
        todo!()
    }
    async fn send_transaction(
        &self,
        transaction: &SentTransaction,
        public_key: String,
        private_key: String,
        account_type: AccountType,
    ) -> Result<(), ServiceError> {
        todo!()
    }
    async fn deploy_token_address_contract(
        &self,
        address: TokenBalanceFromDb,
        public_key: String,
        private_key: String,
        account_type: AccountType,
    ) -> Result<(), ServiceError> {
        todo!()
    }
    async fn get_token_address_info(
        &self,
        address: MsgAddressInt,
        root_address: String,
    ) -> Result<NetworkTokenAddressData, ServiceError> {
        todo!()
    }
    async fn prepare_token_transaction(
        &self,
        transaction: &TokenTransactionSend,
        public_key: String,
        private_key: String,
        account_type: AccountType,
    ) -> Result<SentTokenTransaction, ServiceError> {
        todo!()
    }
    async fn send_token_transaction(
        &self,
        transaction: &SentTokenTransaction,
        public_key: String,
        private_key: String,
        account_type: AccountType,
    ) -> Result<(), ServiceError> {
        todo!()
    }
}
