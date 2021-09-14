use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use bigdecimal::{BigDecimal, ToPrimitive};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer};
use nekoton::core::models::{Expiration, TokenWalletVersion, TransferRecipient};
use nekoton::core::ton_wallet::{MultisigType, TransferAction};
use nekoton::crypto::SignedMessage;
use nekoton_utils::TrustMe;
use num_bigint::BigUint;
use num_traits::FromPrimitive;
use ton_block::{GetRepresentationHash, MsgAddressInt};
use ton_types::UInt256;

use crate::models::*;
use crate::ton_core::*;
use crate::utils::*;

pub use self::responses::*;
pub use self::utils::*;

mod responses;
mod utils;

const DEFAULT_EXPIRATION_TIMEOUT: u32 = 60;
const DEFAULT_MULTISIG_TYPE: MultisigType = MultisigType::SafeMultisigWallet;

#[async_trait]
pub trait TonClient: Send + Sync {
    async fn create_address(&self, payload: CreateAddress) -> Result<CreatedAddress>;
    async fn get_address_info(&self, address: MsgAddressInt) -> Result<NetworkAddressData>;
    async fn get_metrics(&self) -> Result<Metrics>;
    async fn prepare_deploy(
        &self,
        address: &AddressDb,
        public_key: &[u8],
        private_key: &[u8],
    ) -> Result<Option<(SentTransaction, SignedMessage)>>;
    async fn prepare_transaction(
        &self,
        transaction: TransactionSend,
        public_key: &[u8],
        private_key: &[u8],
        account_type: &AccountType,
        custodians: &Option<i32>,
    ) -> Result<(SentTransaction, SignedMessage)>;
    async fn get_token_address_info(
        &self,
        address: &MsgAddressInt,
        root_address: &MsgAddressInt,
    ) -> Result<NetworkTokenAddressData>;
    async fn prepare_token_transaction(
        &self,
        id: uuid::Uuid,
        owner: MsgAddressInt,
        token_wallet: MsgAddressInt,
        destination: TransferRecipient,
        version: TokenWalletVersion,
        tokens: BigDecimal,
        notify_receiver: bool,
        attached_amount: u64,
        account_type: AccountType,
        public_key: &[u8],
        private_key: &[u8],
    ) -> Result<(SentTransaction, SignedMessage)>;
    async fn send_transaction(
        &self,
        account: UInt256,
        signed_message: SignedMessage,
    ) -> Result<MessageStatus>;
}

#[derive(Clone)]
pub struct TonClientImpl {
    ton_core: Arc<TonCore>,
    root_state_cache: RootStateCache,
}

impl TonClientImpl {
    pub fn new(ton_core: Arc<TonCore>, root_state_cache: RootStateCache) -> Self {
        Self {
            ton_core,
            root_state_cache,
        }
    }
}

#[async_trait]
impl TonClient for TonClientImpl {
    async fn create_address(&self, payload: CreateAddress) -> Result<CreatedAddress> {
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
                    DEFAULT_MULTISIG_TYPE,
                    workchain_id as i8,
                )
            }
        };

        let (custodians, confirmations) = match account_type {
            AccountType::SafeMultisig => (
                Some(payload.custodians.unwrap_or(1)),
                Some(payload.confirmations.unwrap_or(1)),
            ),
            AccountType::HighloadWallet | AccountType::Wallet => {
                (payload.custodians, payload.confirmations)
            }
        };

        // Add created account into list of custodians
        let custodians_public_keys = match account_type {
            AccountType::SafeMultisig => {
                let mut custodians_public_keys = payload.custodians_public_keys.unwrap_or_default();
                custodians_public_keys.push(hex::encode(public.to_bytes()));
                Some(custodians_public_keys)
            }
            AccountType::HighloadWallet | AccountType::Wallet => payload.custodians_public_keys,
        };

        // Subscribe to wallets
        {
            let account = UInt256::from_be_bytes(
                &hex::decode(address.address().to_hex_string()).unwrap_or_default(),
            );

            let mut token_accounts = Vec::new();
            let _ = self.root_state_cache.iter().map(|(_, root_state)| {
                let token_account =
                    get_token_wallet_account(root_state.clone(), &address).trust_me();
                token_accounts.push(token_account);
            });

            self.ton_core.add_ton_account_subscription([account]);
            self.ton_core.add_token_account_subscription(token_accounts);
        }

        Ok(CreatedAddress {
            workchain_id: address.workchain_id(),
            hex: address.address().to_hex_string(),
            base64url: nekoton_utils::pack_std_smc_addr(true, &address, false)?,
            public_key: public.to_bytes().to_vec(),
            private_key: secret.to_bytes().to_vec(),
            account_type,
            custodians,
            confirmations,
            custodians_public_keys,
        })
    }
    async fn get_address_info(&self, owner: MsgAddressInt) -> Result<NetworkAddressData> {
        let account = UInt256::from_be_bytes(&owner.address().get_bytestring(0));
        let contract = match self.ton_core.get_contract_state(account).await {
            Ok(contract) => contract,
            Err(_) => return Ok(NetworkAddressData::uninit(&owner)),
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
    async fn get_metrics(&self) -> Result<Metrics> {
        let gen_utime = self.ton_core.get_current_utime();
        Ok(Metrics { gen_utime })
    }
    async fn prepare_deploy(
        &self,
        address: &AddressDb,
        public_key: &[u8],
        private_key: &[u8],
    ) -> Result<Option<(SentTransaction, SignedMessage)>> {
        let public_key = PublicKey::from_bytes(public_key).unwrap_or_default();

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
                    DEFAULT_MULTISIG_TYPE,
                    address.workchain_id as i8,
                    Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT),
                    &owners,
                    address.confirmations.unwrap_or_default() as u8,
                )?
            }
            AccountType::HighloadWallet | AccountType::Wallet => {
                return Ok(None);
            }
        };

        let key_pair = Keypair {
            secret: SecretKey::from_bytes(private_key)?,
            public: public_key,
        };

        let signature = key_pair.sign(unsigned_message.hash());
        let signed_message = unsigned_message.sign(&signature.to_bytes())?;

        let sent_transaction = SentTransaction {
            id: uuid::Uuid::new_v4(),
            message_hash: signed_message.message.hash()?.to_hex_string(),
            account_workchain_id: address.workchain_id,
            account_hex: address.hex.clone(),
            original_value: None,
            original_outputs: None,
            aborted: false,
            bounce: false,
        };

        return Ok(Some((sent_transaction, signed_message)));
    }
    async fn prepare_transaction(
        &self,
        transaction: TransactionSend,
        public_key: &[u8],
        private_key: &[u8],
        account_type: &AccountType,
        custodians: &Option<i32>,
    ) -> Result<(SentTransaction, SignedMessage)> {
        let original_value = transaction.outputs.iter().map(|o| o.value.clone()).sum();
        let original_outputs =
            serde_json::to_value(transaction.outputs.clone()).unwrap_or_default();
        let bounce = transaction.bounce.unwrap_or_default();

        let public_key = PublicKey::from_bytes(public_key).unwrap_or_default();
        let address = nekoton_utils::repack_address(&transaction.from_address.0)?;

        let transfer_action = match account_type {
            AccountType::HighloadWallet => {
                let account = UInt256::from_be_bytes(&address.address().get_bytestring(0));
                let current_state = self.ton_core.get_contract_state(account).await?.account;

                let gifts = transaction
                    .outputs
                    .into_iter()
                    .map(|item| {
                        let flags = item.output_type.unwrap_or_default().value();
                        let destination = nekoton_utils::repack_address(&item.recipient_address.0)?;
                        let amount = item.value.to_u64().unwrap_or_default();

                        Ok(nekoton::core::ton_wallet::highload_wallet_v2::Gift {
                            flags,
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

                nekoton::core::ton_wallet::highload_wallet_v2::prepare_transfer(
                    &public_key,
                    &current_state,
                    gifts,
                    Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT),
                )?
            }
            AccountType::Wallet => {
                let account = UInt256::from_be_bytes(&address.address().get_bytestring(0));
                let current_state = self.ton_core.get_contract_state(account).await?.account;

                let recipient = transaction
                    .outputs
                    .first()
                    .ok_or(TonClientError::RecipientNotFound)?;

                let destination = nekoton_utils::repack_address(&recipient.recipient_address.0)?;
                let amount = recipient.value.clone();
                let amount = amount.to_u64().unwrap_or_default();

                nekoton::core::ton_wallet::wallet_v3::prepare_transfer(
                    &public_key,
                    &current_state,
                    destination,
                    amount,
                    bounce,
                    None,
                    Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT),
                )?
            }
            AccountType::SafeMultisig => {
                let recipient = transaction
                    .outputs
                    .first()
                    .ok_or(TonClientError::RecipientNotFound)?;

                let destination = nekoton_utils::repack_address(&recipient.recipient_address.0)?;
                let amount = recipient.value.to_u64().unwrap_or_default();

                let has_multiple_owners = match custodians {
                    Some(custodians) => *custodians > 1,
                    None => return Err(TonClientError::CustodiansNotFound.into()),
                };

                nekoton::core::ton_wallet::multisig::prepare_transfer(
                    &public_key,
                    has_multiple_owners,
                    address.clone(),
                    destination,
                    amount,
                    bounce,
                    None,
                    Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT),
                )?
            }
        };

        let unsigned_message = match transfer_action {
            TransferAction::Sign(unsigned_message) => unsigned_message,
            TransferAction::DeployFirst => {
                return Err(TonClientError::AccountNotDeployed(address.to_string()).into())
            }
        };

        let key_pair = Keypair {
            secret: SecretKey::from_bytes(private_key)?,
            public: public_key,
        };

        let signature = key_pair.sign(unsigned_message.hash());
        let signed_message = unsigned_message.sign(&signature.to_bytes())?;

        let sent_transaction = SentTransaction {
            id: transaction.id,
            message_hash: signed_message.message.hash()?.to_hex_string(),
            account_workchain_id: address.workchain_id(),
            account_hex: address.address().to_hex_string(),
            original_value: Some(original_value),
            original_outputs: Some(original_outputs),
            aborted: false,
            bounce,
        };

        Ok((sent_transaction, signed_message))
    }
    async fn get_token_address_info(
        &self,
        owner: &MsgAddressInt,
        root_address: &MsgAddressInt,
    ) -> Result<NetworkTokenAddressData> {
        let root_contract = self
            .root_state_cache
            .get(root_address)
            .ok_or(TonClientError::UnknownRootContract)?;

        let token_address = get_token_wallet_address(root_contract.clone(), owner)?;
        let token_account = UInt256::from_be_bytes(&token_address.address().get_bytestring(0));
        let token_contract = match self.ton_core.get_contract_state(token_account).await {
            Ok(contract) => contract,
            Err(_) => {
                return Ok(NetworkTokenAddressData::uninit(
                    &token_address,
                    root_address,
                ))
            }
        };

        let (version, network_balance) = get_token_wallet_details(&token_contract)?;
        let account_status = transform_account_state(&token_contract.account.storage.state);
        let sync_u_time = token_contract.timings.current_utime() as i64;

        let (last_transaction_hash, last_transaction_lt) =
            parse_last_transaction(&token_contract.last_transaction_id);

        Ok(NetworkTokenAddressData {
            workchain_id: token_address.workchain_id(),
            hex: token_address.address().to_hex_string(),
            root_address: root_address.to_string(),
            version: version.to_string(),
            network_balance,
            account_status,
            last_transaction_hash,
            last_transaction_lt,
            sync_u_time,
        })
    }
    async fn prepare_token_transaction(
        &self,
        id: uuid::Uuid,
        owner: MsgAddressInt,
        token_wallet: MsgAddressInt,
        destination: TransferRecipient,
        version: TokenWalletVersion,
        tokens: BigDecimal,
        notify_receiver: bool,
        attached_amount: u64,
        account_type: AccountType,
        public_key: &[u8],
        private_key: &[u8],
    ) -> Result<(SentTransaction, SignedMessage)> {
        let tokens = BigUint::from_u64(tokens.to_u64().unwrap_or_default()).unwrap_or_default();

        let internal_message = prepare_token_transfer(
            owner.clone(),
            token_wallet,
            destination,
            version,
            tokens,
            notify_receiver,
            attached_amount,
            Default::default(),
        )?;

        let bounce = internal_message.bounce;
        let destination = internal_message.destination;
        let amount = internal_message.amount;
        let body = Some(internal_message.body);
        let expiration = Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT);

        let public_key = PublicKey::from_bytes(public_key).unwrap_or_default();

        let transfer_action = match account_type {
            AccountType::HighloadWallet => {
                let account = UInt256::from_be_bytes(&owner.address().get_bytestring(0));
                let current_state = self.ton_core.get_contract_state(account).await?.account;

                let gift = nekoton::core::ton_wallet::highload_wallet_v2::Gift {
                    flags: TransactionSendOutputType::Normal.value(),
                    bounce,
                    destination,
                    amount,
                    body,
                    state_init: None,
                };

                nekoton::core::ton_wallet::highload_wallet_v2::prepare_transfer(
                    &public_key,
                    &current_state,
                    vec![gift],
                    expiration,
                )?
            }
            AccountType::Wallet => {
                let account = UInt256::from_be_bytes(&owner.address().get_bytestring(0));
                let current_state = self.ton_core.get_contract_state(account).await?.account;

                nekoton::core::ton_wallet::wallet_v3::prepare_transfer(
                    &public_key,
                    &current_state,
                    destination,
                    amount,
                    bounce,
                    body,
                    expiration,
                )?
            }
            AccountType::SafeMultisig => {
                let has_multiple_owners = false; // TODO

                nekoton::core::ton_wallet::multisig::prepare_transfer(
                    &public_key,
                    has_multiple_owners,
                    owner.clone(),
                    destination,
                    amount,
                    bounce,
                    body,
                    expiration,
                )?
            }
        };

        let unsigned_message = match transfer_action {
            TransferAction::Sign(unsigned_message) => unsigned_message,
            TransferAction::DeployFirst => {
                return Err(TonClientError::AccountNotDeployed(owner.to_string()).into())
            }
        };

        let key_pair = Keypair {
            secret: SecretKey::from_bytes(private_key)?,
            public: public_key,
        };

        let signature = key_pair.sign(unsigned_message.hash());
        let signed_message = unsigned_message.sign(&signature.to_bytes())?;

        let sent_transaction = SentTransaction {
            id,
            message_hash: signed_message.message.hash()?.to_hex_string(),
            account_workchain_id: owner.workchain_id(),
            account_hex: owner.address().to_hex_string(),
            original_value: None,
            original_outputs: None,
            aborted: false,
            bounce: false,
        };

        return Ok((sent_transaction, signed_message));
    }
    async fn send_transaction(
        &self,
        account: UInt256,
        signed_message: SignedMessage,
    ) -> Result<MessageStatus> {
        self.ton_core
            .send_ton_message(&account, &signed_message.message, signed_message.expire_at)
            .await
    }
}

#[derive(thiserror::Error, Debug)]
enum TonClientError {
    #[error("Recipient is empty")]
    RecipientNotFound,
    #[error("Account `{0}` not deployed")]
    AccountNotDeployed(String),
    #[error("Custodians not found")]
    CustodiansNotFound,
    #[error("Unknown root contract")]
    UnknownRootContract,
}
