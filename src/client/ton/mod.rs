use std::convert::TryFrom;
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use bigdecimal::BigDecimal;
use ed25519_dalek::{Keypair, Signer};
use nekoton::core::models::RootTokenContractDetails;
use nekoton::core::token_wallet::{RootTokenContractState, TokenWalletContractState};
use nekoton_abi::LastTransactionId;
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

    fn parse_account_state(&self, account_state: &AccountState) -> AccountStatus {
        match account_state {
            AccountState::AccountUninit => AccountStatus::UnInit,
            AccountState::AccountActive(_) => AccountStatus::Active,
            AccountState::AccountFrozen(_) => AccountStatus::Frozen,
        }
    }

    fn parse_last_transaction(
        &self,
        last_transaction: &LastTransactionId,
    ) -> (Option<String>, Option<String>) {
        let (last_transaction_hash, last_transaction_lt) = match last_transaction {
            LastTransactionId::Exact(transaction_id) => (
                Some(transaction_id.hash.to_hex_string()),
                Some(transaction_id.lt.to_string()),
            ),
            LastTransactionId::Inexact { .. } => (None, None),
        };

        (last_transaction_hash, last_transaction_lt)
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
        let contract = self.ton_core.get_contract_state(account).await?;

        let account_status = self.parse_account_state(&contract.account.storage.state);
        let network_balance = BigDecimal::from_u128(contract.account.storage.balance.grams.0)
            .ok_or_else(|| {
                ServiceError::Other(anyhow::anyhow!(
                    "Failed to get balance for account `{}`",
                    account.to_hex_string()
                ))
            })?;

        let (last_transaction_hash, last_transaction_lt) =
            self.parse_last_transaction(&contract.last_transaction_id);

        Ok(NetworkAddressData {
            workchain_id: contract.account.addr.workchain_id(),
            hex: contract.account.addr.address().to_hex_string(),
            account_status,
            network_balance,
            last_transaction_hash,
            last_transaction_lt,
            sync_u_time: 0,
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
        let root_account = UInt256::from_be_bytes(
            &MsgAddressInt::from_str(&root_address)?
                .address()
                .get_bytestring(0),
        );
        let root_contract = self.ton_core.get_contract_state(root_account).await?;

        let root_contract_state = RootTokenContractState(&root_contract);
        let RootTokenContractDetails { version, .. } = root_contract_state.guess_details()?;

        let token_wallet_address =
            root_contract_state.get_wallet_address(version, &address, None)?;

        let token_wallet_account =
            UInt256::from_be_bytes(&token_wallet_address.address().get_bytestring(0));
        let token_contract = self
            .ton_core
            .get_contract_state(token_wallet_account)
            .await?;

        let token_wallet = TokenWalletContractState(&token_contract);
        let version = token_wallet.get_version()?;

        let network_balance = BigDecimal::new(token_wallet.get_balance(version)?.into(), 0); // TODO
        let account_status = self.parse_account_state(&token_contract.account.storage.state);

        let (last_transaction_hash, last_transaction_lt) =
            self.parse_last_transaction(&token_contract.last_transaction_id);

        Ok(NetworkTokenAddressData {
            workchain_id: token_wallet_address.workchain_id(),
            hex: token_wallet_address.address().to_hex_string(),
            root_address,
            network_balance,
            account_status,
            last_transaction_hash,
            last_transaction_lt,
            sync_u_time: 0,
        })
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
