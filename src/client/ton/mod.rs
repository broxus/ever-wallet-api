use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use bigdecimal::{BigDecimal, ToPrimitive};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer};
use nekoton::core::models::{Expiration, RootTokenContractDetails};
use nekoton::core::token_wallet::{RootTokenContractState, TokenWalletContractState};
use nekoton::core::ton_wallet::{MultisigType, TransferAction, WalletType};
use nekoton::crypto::UnsignedMessage;
use num_traits::FromPrimitive;
use ton_block::MsgAddressInt;
use ton_types::UInt256;

use crate::models::account_enums::AccountType;
use crate::models::address::{CreateAddress, NetworkAddressData};
use crate::models::sqlx::TokenBalanceFromDb;
use crate::models::token_balance::NetworkTokenAddressData;
use crate::models::token_transactions::TokenTransactionSend;
use crate::models::transactions::TransactionSend;
use crate::prelude::ServiceError;
use crate::ton_core::TonCore;

pub use self::models::*;
pub use self::responses::*;
pub use self::utils::*;

mod models;
mod responses;
mod utils;

#[async_trait]
pub trait TonClient: Send + Sync {
    async fn create_address(&self, payload: &CreateAddress)
        -> Result<CreatedAddress, ServiceError>;
    async fn get_address_info(
        &self,
        address: MsgAddressInt,
    ) -> Result<NetworkAddressData, ServiceError>;
    async fn deploy_address_contract(&self, address: AddressDeploy) -> Result<(), ServiceError>;
    async fn get_token_address_info(
        &self,
        address: MsgAddressInt,
        root_address: MsgAddressInt,
    ) -> Result<NetworkTokenAddressData, ServiceError>;
    async fn prepare_transaction(
        &self,
        transaction: &TransactionSend,
        public_key: &[u8],
        account_type: AccountType,
    ) -> Result<(SentTransaction, Box<dyn UnsignedMessage>), ServiceError>;
    async fn send_transaction(
        &self,
        unsigned_message: Box<dyn UnsignedMessage>,
        public_key: &[u8],
        private_key: &[u8],
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

pub const DEFAULT_EXPIRATION_TIMEOUT: u32 = 60;
pub const MULTISIG_TYPE: MultisigType = MultisigType::SafeMultisigWallet;

#[async_trait]
impl TonClient for TonClientImpl {
    async fn create_address(
        &self,
        payload: &CreateAddress,
    ) -> Result<CreatedAddress, ServiceError> {
        let generated_key = nekoton::crypto::generate_key(nekoton::crypto::MnemonicType::Labs(0))?;

        let Keypair { public, secret } = nekoton::crypto::derive_from_phrase(
            &generated_key.words.join(" "),
            generated_key.account_type,
        )?;

        let workchain_id = payload.workchain_id.unwrap_or_default();
        let account_type = payload.account_type.unwrap_or(AccountType::HighloadWallet);

        let address = match account_type {
            AccountType::HighloadWallet => {
                nekoton::core::ton_wallet::highload_wallet_v2::compute_contract_address(
                    &public,
                    workchain_id as i8,
                )
            }
            AccountType::Wallet => nekoton::core::ton_wallet::wallet_v3::compute_contract_address(
                &public,
                workchain_id as i8,
            ),
            AccountType::SafeMultisig => {
                nekoton::core::ton_wallet::multisig::compute_contract_address(
                    &public,
                    MULTISIG_TYPE,
                    workchain_id as i8,
                )
            }
        };

        Ok(CreatedAddress {
            workchain_id: address.workchain_id(),
            hex: address.address().to_hex_string(),
            base64url: nekoton_utils::pack_std_smc_addr(true, &address, false)?,
            public_key: hex::encode(public.to_bytes()),
            private_key: hex::encode(secret.to_bytes()),
            account_type,
            custodians: payload.custodians,
            confirmations: payload.confirmations,
            custodians_public_keys: payload.custodians_public_keys.clone(), // TODO
        })
    }
    async fn get_address_info(
        &self,
        address: MsgAddressInt,
    ) -> Result<NetworkAddressData, ServiceError> {
        let account = UInt256::from_be_bytes(&address.address().get_bytestring(0));
        let contract = self.ton_core.get_contract_state(account).await?;

        let account_status = transform_account_state(&contract.account.storage.state);
        let network_balance =
            BigDecimal::from_u128(contract.account.storage.balance.grams.0).unwrap_or_default(); // TODO

        let (last_transaction_hash, last_transaction_lt) =
            parse_last_transaction(&contract.last_transaction_id);

        Ok(NetworkAddressData {
            workchain_id: contract.account.addr.workchain_id(),
            hex: contract.account.addr.address().to_hex_string(),
            account_status,
            network_balance,
            last_transaction_hash,
            last_transaction_lt,
            sync_u_time: contract.timings.current_utime() as i64,
        })
    }
    async fn deploy_address_contract(&self, address: AddressDeploy) -> Result<(), ServiceError> {
        let expiration = Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT);

        let (public_key, secret, unsigned_message) = match address {
            AddressDeploy::HighloadWallet(wallet) => {
                let public_key = PublicKey::from_bytes(&wallet.public_key)
                    .map_err(|err| ServiceError::Other(err.into()))?;

                let secret = SecretKey::from_bytes(&wallet.secret)
                    .map_err(|err| ServiceError::Other(err.into()))?;

                let unsigned_message =
                    nekoton::core::ton_wallet::highload_wallet_v2::prepare_deploy(
                        &public_key,
                        wallet.workchain,
                        expiration,
                    )?;

                (public_key, secret, unsigned_message)
            }
            AddressDeploy::WalletV3(wallet) => {
                let public_key = PublicKey::from_bytes(&wallet.public_key)
                    .map_err(|err| ServiceError::Other(err.into()))?;

                let secret = SecretKey::from_bytes(&wallet.secret)
                    .map_err(|err| ServiceError::Other(err.into()))?;

                let unsigned_message = nekoton::core::ton_wallet::wallet_v3::prepare_deploy(
                    &public_key,
                    wallet.workchain,
                    expiration,
                )?;

                (public_key, secret, unsigned_message)
            }
            AddressDeploy::SafeMultisig(wallet) => {
                let public_key = PublicKey::from_bytes(&wallet.public_key)
                    .map_err(|err| ServiceError::Other(err.into()))?;

                let secret = SecretKey::from_bytes(&wallet.secret)
                    .map_err(|err| ServiceError::Other(err.into()))?;

                let mut owners = Vec::new();
                for owner in wallet.owners {
                    owners.push(
                        PublicKey::from_bytes(&owner)
                            .map_err(|err| ServiceError::Other(err.into()))?,
                    );
                }

                let unsigned_message = nekoton::core::ton_wallet::multisig::prepare_deploy(
                    &public_key,
                    MULTISIG_TYPE,
                    wallet.workchain,
                    expiration,
                    &owners,
                    wallet.req_confirms,
                )?;

                (public_key, secret, unsigned_message)
            }
        };

        let key_pair = Keypair {
            secret,
            public: public_key,
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
        public_key: &[u8],
        account_type: AccountType,
    ) -> Result<(SentTransaction, Box<dyn UnsignedMessage>), ServiceError> {
        let public_key =
            PublicKey::from_bytes(&public_key).map_err(|err| ServiceError::Other(err.into()))?;

        let address = MsgAddressInt::from_str(&transaction.from_address.0)?;
        let account = UInt256::from_be_bytes(&address.address().get_bytestring(0));
        let state = self.ton_core.get_contract_state(account).await?;

        let mut amount = Default::default();
        let transfer_action = match account_type {
            AccountType::HighloadWallet => {
                todo!()
            }
            AccountType::Wallet => {
                let recipient = transaction
                    .outputs
                    .first()
                    .ok_or_else(|| ServiceError::Other(TonClientError::InvalidRecipient.into()))?;

                let dst = MsgAddressInt::from_str(&recipient.recipient_address.0)?;
                amount = recipient.value.to_u64().unwrap_or_default();

                nekoton::core::ton_wallet::wallet_v3::prepare_transfer(
                    &public_key,
                    &state.account,
                    dst,
                    amount,
                    false,
                    None,
                    Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT),
                )?
            }
            AccountType::SafeMultisig => {
                todo!()
            }
        };

        let unsigned_message = match transfer_action {
            TransferAction::Sign(unsigned_message) => unsigned_message,
            TransferAction::DeployFirst => {
                return Err(ServiceError::Other(
                    TonClientError::AccountNotDeployed(address.to_string()).into(),
                ))
            }
        };

        let sent_transaction = SentTransaction {
            id: transaction.id,
            message_hash: hex::encode(unsigned_message.hash()),
            account_workchain_id: address.workchain_id(),
            account_hex: address.address().to_hex_string(),
            value: BigDecimal::from_u64(amount).unwrap_or_default(),
            aborted: false,
            bounce: false,
        };

        Ok((sent_transaction, unsigned_message))
    }
    async fn send_transaction(
        &self,
        unsigned_message: Box<dyn UnsignedMessage>,
        public_key: &[u8],
        private_key: &[u8],
    ) -> Result<(), ServiceError> {
        let public =
            PublicKey::from_bytes(public_key).map_err(|err| ServiceError::Other(err.into()))?;
        let secret =
            SecretKey::from_bytes(private_key).map_err(|err| ServiceError::Other(err.into()))?;

        let key_pair = Keypair { secret, public };

        let signature = key_pair.sign(unsigned_message.hash());
        let signed_message = unsigned_message.sign(&signature.to_bytes())?;

        self.ton_core
            .send_ton_message(&signed_message.message, signed_message.expire_at)
            .await
            .map_err(ServiceError::Other)
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
        root_address: MsgAddressInt,
    ) -> Result<NetworkTokenAddressData, ServiceError> {
        let root_account = UInt256::from_be_bytes(&root_address.address().get_bytestring(0));
        let root_contract = self.ton_core.get_contract_state(root_account).await?;
        let root_contract_state = RootTokenContractState(&root_contract);

        let RootTokenContractDetails { version, .. } = root_contract_state.guess_details()?;

        let token_wallet_address =
            root_contract_state.get_wallet_address(version, &address, None)?;
        let token_wallet_account =
            UInt256::from_be_bytes(&token_wallet_address.address().get_bytestring(0));
        let token_wallet_contract_state = self
            .ton_core
            .get_contract_state(token_wallet_account)
            .await?;

        let token_wallet = TokenWalletContractState(&token_wallet_contract_state);
        let version = token_wallet.get_version()?;

        let network_balance = BigDecimal::new(token_wallet.get_balance(version)?.into(), 0); // TODO
        let account_status =
            transform_account_state(&token_wallet_contract_state.account.storage.state);

        let (last_transaction_hash, last_transaction_lt) =
            parse_last_transaction(&token_wallet_contract_state.last_transaction_id);

        Ok(NetworkTokenAddressData {
            workchain_id: token_wallet_address.workchain_id(),
            hex: token_wallet_address.address().to_hex_string(),
            root_address: root_address.to_string(),
            network_balance,
            account_status,
            last_transaction_hash,
            last_transaction_lt,
            sync_u_time: token_wallet_contract_state.timings.current_utime() as i64,
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

#[derive(thiserror::Error, Debug)]
enum TonClientError {
    #[error("Recipient not found")]
    InvalidRecipient,
    #[error("Account `{0}` not deployed")]
    AccountNotDeployed(String),
}
