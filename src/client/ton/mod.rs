use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use bigdecimal::{BigDecimal, ToPrimitive};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer};
use nekoton::core::models::{Expiration, RootTokenContractDetails};
use nekoton::core::token_wallet::{RootTokenContractState, TokenWalletContractState};
use nekoton::core::ton_wallet::{MultisigType, TransferAction};
use nekoton::crypto::SignedMessage;
use num_traits::FromPrimitive;
use ton_block::{GetRepresentationHash, MsgAddressInt};
use ton_types::UInt256;

use crate::models::*;
use crate::prelude::*;
use crate::ton_core::*;
use crate::utils::*;

pub use self::responses::*;
pub use self::utils::*;

mod responses;
mod utils;

pub const DEFAULT_EXPIRATION_TIMEOUT: u32 = 60;
pub const MULTISIG_TYPE: MultisigType = MultisigType::SafeMultisigWallet;

#[async_trait]
pub trait TonClient: Send + Sync {
    async fn create_address(&self, payload: CreateAddress) -> Result<CreatedAddress, ServiceError>;
    async fn get_address_info(
        &self,
        address: MsgAddressInt,
    ) -> Result<NetworkAddressData, ServiceError>;
    async fn deploy_address_contract(
        &self,
        address: &AddressDb,
        secret: &[u8],
    ) -> Result<MessageStatus, ServiceError>;
    async fn get_token_address_info(
        &self,
        address: MsgAddressInt,
        root_address: MsgAddressInt,
    ) -> Result<NetworkTokenAddressData, ServiceError>;
    async fn prepare_transaction(
        &self,
        transaction: TransactionSend,
        public_key: &[u8],
        private_key: &[u8],
        account_type: &AccountType,
        custodians: &Option<i32>,
    ) -> Result<(SentTransaction, SignedMessage), ServiceError>;
    async fn send_transaction(
        &self,
        account: UInt256,
        signed_message: SignedMessage,
    ) -> Result<MessageStatus, ServiceError>;
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
    async fn create_address(&self, payload: CreateAddress) -> Result<CreatedAddress, ServiceError> {
        let generated_key = nekoton::crypto::generate_key(nekoton::crypto::MnemonicType::Labs(0))?;

        let Keypair { public, secret } = nekoton::crypto::derive_from_phrase(
            &generated_key.words.join(" "),
            generated_key.account_type,
        )?;

        let workchain_id = payload.workchain_id.unwrap_or_default();
        let account_type = payload.account_type.unwrap_or_default();

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

        let custodians_public_keys = match account_type {
            AccountType::SafeMultisig => {
                let mut custodians_public_keys = payload.custodians_public_keys.unwrap_or_default();
                custodians_public_keys.push(hex::encode(public.to_bytes()));
                Some(custodians_public_keys)
            }
            AccountType::HighloadWallet | AccountType::Wallet => payload.custodians_public_keys,
        };

        let account = UInt256::from_be_bytes(
            &hex::decode(address.address().to_hex_string()).unwrap_or_default(),
        );
        self.ton_core.add_account_subscription([account]);

        Ok(CreatedAddress {
            workchain_id: address.workchain_id(),
            hex: address.address().to_hex_string(),
            base64url: nekoton_utils::pack_std_smc_addr(true, &address, false)?,
            public_key: public.to_bytes().to_vec(),
            private_key: secret.to_bytes().to_vec(),
            account_type,
            custodians: payload.custodians,
            confirmations: payload.confirmations,
            custodians_public_keys,
        })
    }
    async fn get_address_info(
        &self,
        address: MsgAddressInt,
    ) -> Result<NetworkAddressData, ServiceError> {
        let account = UInt256::from_be_bytes(&address.address().get_bytestring(0));
        let contract = match self.ton_core.get_contract_state(account).await {
            Ok(contract) => contract,
            Err(_) => {
                return Ok(NetworkAddressData {
                    workchain_id: address.workchain_id(),
                    hex: address.address().to_hex_string(),
                    account_status: AccountStatus::UnInit,
                    network_balance: Default::default(),
                    last_transaction_hash: None,
                    last_transaction_lt: None,
                    sync_u_time: Default::default(),
                })
            }
        };

        let account_status = transform_account_state(&contract.account.storage.state);
        let network_balance =
            BigDecimal::from_u128(contract.account.storage.balance.grams.0).unwrap_or_default();

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
    async fn deploy_address_contract(
        &self,
        address: &AddressDb,
        private_key: &[u8],
    ) -> Result<MessageStatus, ServiceError> {
        let public_key =
            PublicKey::from_bytes(&hex::decode(&address.public_key).unwrap_or_default())
                .unwrap_or_default();

        let unsigned_message = match address.account_type {
            AccountType::SafeMultisig => {
                let owners: Vec<String> = address
                    .custodians_public_keys
                    .clone()
                    .map(|pks| serde_json::from_value(pks).unwrap_or_default())
                    .unwrap_or_default();
                let owners = owners
                    .into_iter()
                    .map(|o| hex::decode(o).unwrap_or_default())
                    .collect::<Vec<Vec<u8>>>();
                let mut owners = owners
                    .into_iter()
                    .map(|item| PublicKey::from_bytes(&item).unwrap_or_default())
                    .collect::<Vec<PublicKey>>();
                owners.push(public_key);
                nekoton::core::ton_wallet::multisig::prepare_deploy(
                    &public_key,
                    MULTISIG_TYPE,
                    address.workchain_id as i8,
                    Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT),
                    &owners,
                    address.confirmations.unwrap_or_default() as u8,
                )?
            }
            AccountType::HighloadWallet | AccountType::Wallet => {
                return Ok(MessageStatus::Delivered);
            }
        };

        let secret =
            SecretKey::from_bytes(private_key).map_err(|err| ServiceError::Other(err.into()))?;

        let key_pair = Keypair {
            secret,
            public: public_key,
        };

        let signature = key_pair.sign(unsigned_message.hash());
        let signed_message = unsigned_message.sign(&signature.to_bytes())?;

        let account = UInt256::from_be_bytes(&hex::decode(&address.hex).unwrap_or_default());

        self.ton_core
            .send_ton_message(&account, &signed_message.message, signed_message.expire_at)
            .await
            .map_err(ServiceError::Other)
    }
    async fn prepare_transaction(
        &self,
        transaction: TransactionSend,
        public_key: &[u8],
        private_key: &[u8],
        account_type: &AccountType,
        custodians: &Option<i32>,
    ) -> Result<(SentTransaction, SignedMessage), ServiceError> {
        let public_key = PublicKey::from_bytes(public_key).unwrap_or_default();

        let address = nekoton_utils::repack_address(&transaction.from_address.0)?;
        let bounce = transaction.bounce.unwrap_or_default();

        let (transfer_action, amount) = match account_type {
            AccountType::HighloadWallet => {
                let account = UInt256::from_be_bytes(&address.address().get_bytestring(0));
                let current_state = self.ton_core.get_contract_state(account).await?.account;

                let gifts = transaction
                    .outputs
                    .into_iter()
                    .map(|item| {
                        let destination = nekoton_utils::repack_address(&item.recipient_address.0)?;
                        let amount = item.value.to_u64().unwrap_or_default();

                        Ok(nekoton::core::ton_wallet::highload_wallet_v2::Gift {
                            flags: 0,
                            bounce,
                            destination,
                            amount,
                            body: None,
                            state_init: None,
                        })
                    })
                    .collect::<Vec<Result<nekoton::core::ton_wallet::highload_wallet_v2::Gift>>>();

                let gifts = gifts
                    .into_iter()
                    .collect::<Result<Vec<nekoton::core::ton_wallet::highload_wallet_v2::Gift>>>(
                    )?;

                let amount = gifts.iter().map(|gift| gift.amount).sum();

                (
                    nekoton::core::ton_wallet::highload_wallet_v2::prepare_transfer(
                        &public_key,
                        &current_state,
                        gifts,
                        Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT),
                    )?,
                    amount,
                )
            }
            AccountType::Wallet => {
                let account = UInt256::from_be_bytes(&address.address().get_bytestring(0));
                let current_state = self.ton_core.get_contract_state(account).await?.account;

                let recipient = transaction
                    .outputs
                    .first()
                    .ok_or_else(|| ServiceError::Other(TonClientError::InvalidRecipient.into()))?;

                let destination = nekoton_utils::repack_address(&recipient.recipient_address.0)?;
                let amount = recipient.value.to_u64().unwrap_or_default();

                (
                    nekoton::core::ton_wallet::wallet_v3::prepare_transfer(
                        &public_key,
                        &current_state,
                        destination,
                        amount,
                        bounce,
                        None,
                        Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT),
                    )?,
                    amount,
                )
            }
            AccountType::SafeMultisig => {
                let recipient = transaction
                    .outputs
                    .first()
                    .ok_or_else(|| ServiceError::Other(TonClientError::InvalidRecipient.into()))?;

                let destination = nekoton_utils::repack_address(&recipient.recipient_address.0)?;
                let amount = recipient.value.to_u64().unwrap_or_default();

                let has_multiple_owners = match custodians {
                    Some(custodians) => *custodians > 1,
                    None => {
                        return Err(ServiceError::Other(
                            TonClientError::CustodiansNotFound.into(),
                        ))
                    }
                };

                (
                    nekoton::core::ton_wallet::multisig::prepare_transfer(
                        &public_key,
                        has_multiple_owners,
                        address.clone(),
                        destination,
                        amount,
                        bounce,
                        None,
                        Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT),
                    )?,
                    amount,
                )
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

        let secret =
            SecretKey::from_bytes(private_key).map_err(|err| ServiceError::Other(err.into()))?;

        let key_pair = Keypair {
            secret,
            public: public_key,
        };

        let signature = key_pair.sign(unsigned_message.hash());
        let signed_message = unsigned_message.sign(&signature.to_bytes())?;

        let sent_transaction = SentTransaction {
            id: transaction.id,
            message_hash: signed_message.message.hash()?.to_hex_string(),
            account_workchain_id: address.workchain_id(),
            account_hex: address.address().to_hex_string(),
            value: BigDecimal::from_u64(amount).unwrap_or_default(),
            aborted: false,
            bounce,
        };

        Ok((sent_transaction, signed_message))
    }
    async fn send_transaction(
        &self,
        account: UInt256,
        signed_message: SignedMessage,
    ) -> Result<MessageStatus, ServiceError> {
        self.ton_core
            .send_ton_message(&account, &signed_message.message, signed_message.expire_at)
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

        let network_balance = BigDecimal::new(token_wallet.get_balance(version)?.into(), 0);
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
    #[error("Custodians not found")]
    CustodiansNotFound,
}
