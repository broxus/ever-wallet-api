use std::sync::Arc;

use anyhow::{Error, Result};
use async_trait::async_trait;
use bigdecimal::{BigDecimal, ToPrimitive};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer};
use nekoton::core::models::Expiration;
use nekoton::core::ton_wallet::{MultisigType, TransferAction};
use nekoton::core::InternalMessage;
use nekoton::crypto::SignedMessage;
use nekoton_utils::{repack_address, SimpleClock, TrustMe};
use num_bigint::BigUint;
use num_traits::FromPrimitive;
use tokio::sync::oneshot;
use ton_block::{GetRepresentationHash, MsgAddressInt};
use ton_types::UInt256;
use uuid::Uuid;

use crate::models::*;
use crate::sqlx_client::*;
use crate::ton_core::*;
use crate::utils::*;

pub use self::utils::*;

mod utils;

pub const DEFAULT_EXPIRATION_TIMEOUT: u32 = 60;

const DEFAULT_MULTISIG_TYPE: MultisigType = MultisigType::SafeMultisigWallet;

#[async_trait]
pub trait TonClient: Send + Sync {
    async fn create_address(&self, payload: CreateAddress) -> Result<CreatedAddress>;
    async fn get_address_info(&self, address: &MsgAddressInt) -> Result<NetworkAddressData>;
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
    async fn prepare_confirm_transaction(
        &self,
        transaction: TransactionConfirm,
        public_key: &[u8],
        private_key: &[u8],
    ) -> Result<(SentTransaction, SignedMessage)>;
    async fn get_token_address_info(
        &self,
        address: &MsgAddressInt,
        root_address: &MsgAddressInt,
    ) -> Result<NetworkTokenAddressData>;
    async fn prepare_token_transaction(
        &self,
        input: &TokenTransactionSend,
        public_key: &[u8],
        private_key: &[u8],
        account_type: &AccountType,
        custodians: &Option<i32>,
    ) -> Result<(SentTransaction, SignedMessage)>;
    async fn prepare_token_burn(
        &self,
        input: &TokenTransactionBurn,
        public_key: &[u8],
        private_key: &[u8],
        account_type: &AccountType,
        custodians: &Option<i32>,
    ) -> Result<(SentTransaction, SignedMessage)>;
    async fn prepare_token_mint(
        &self,
        input: &TokenTransactionMint,
        public_key: &[u8],
        private_key: &[u8],
        account_type: &AccountType,
        custodians: &Option<i32>,
    ) -> Result<(SentTransaction, SignedMessage)>;
    async fn send_transaction(
        &self,
        account: UInt256,
        signed_message: SignedMessage,
    ) -> Result<MessageStatus>;
    fn add_pending_message(
        &self,
        account: UInt256,
        message_hash: UInt256,
        expire_at: u32,
    ) -> Result<oneshot::Receiver<MessageStatus>>;
    async fn get_metrics(&self) -> Result<Metrics>;
}

#[derive(Clone)]
pub struct TonClientImpl {
    ton_core: Arc<TonCore>,
    sqlx_client: SqlxClient,
}

impl TonClientImpl {
    pub fn new(ton_core: Arc<TonCore>, sqlx_client: SqlxClient) -> Self {
        Self {
            ton_core,
            sqlx_client,
        }
    }

    pub async fn start(&self) -> Result<()> {
        let owner_addresses = self
            .sqlx_client
            .get_all_addresses()
            .await?
            .into_iter()
            .map(|item| {
                nekoton_utils::repack_address(&format!("{}:{}", item.workchain_id, item.hex))
                    .trust_me()
            })
            .collect::<Vec<MsgAddressInt>>();

        // Subscribe to ton accounts
        let owner_accounts = owner_addresses
            .iter()
            .map(|item| UInt256::from_be_bytes(&item.address().get_bytestring(0)))
            .collect::<Vec<UInt256>>();

        self.ton_core.add_ton_account_subscription(owner_accounts);

        // Add token observer
        self.ton_core.init_token_subscription();

        Ok(())
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
        let account = UInt256::from_be_bytes(&hex::decode(address.address().to_hex_string())?);
        self.ton_core.add_ton_account_subscription([account]);

        Ok(CreatedAddress {
            workchain_id: address.workchain_id(),
            hex: address.address().to_hex_string(),
            base64url: nekoton_utils::pack_std_smc_addr(true, &address, true)?,
            public_key: public.to_bytes().to_vec(),
            private_key: secret.to_bytes().to_vec(),
            account_type,
            custodians,
            confirmations,
            custodians_public_keys,
        })
    }
    async fn get_address_info(&self, owner: &MsgAddressInt) -> Result<NetworkAddressData> {
        let account = UInt256::from_be_bytes(&owner.address().get_bytestring(0));
        let contract = match self.ton_core.get_contract_state(&account) {
            Ok(contract) => contract,
            Err(_) => return Ok(NetworkAddressData::uninit(owner)),
        };

        let network_balance = BigDecimal::from_u128(contract.account.storage.balance.grams.0)
            .ok_or(TonClientError::ParseBigDecimal)?;

        let (last_transaction_hash, last_transaction_lt) =
            parse_last_transaction(&contract.last_transaction_id);

        Ok(NetworkAddressData {
            workchain_id: contract.account.addr.workchain_id(),
            hex: contract.account.addr.address().to_hex_string(),
            account_status: contract.account.storage.state.into(),
            network_balance,
            last_transaction_hash,
            last_transaction_lt,
            sync_u_time: contract.timings.current_utime(&SimpleClock) as i64,
        })
    }

    async fn prepare_deploy(
        &self,
        address: &AddressDb,
        public_key: &[u8],
        private_key: &[u8],
    ) -> Result<Option<(SentTransaction, SignedMessage)>> {
        let public_key = PublicKey::from_bytes(public_key)?;

        let unsigned_message = match address.account_type {
            AccountType::SafeMultisig => {
                let owners: Vec<String> = address
                    .custodians_public_keys
                    .clone()
                    .map(|pks| serde_json::from_value(pks).unwrap_or_default())
                    .unwrap_or_default();
                let mut owners = owners
                    .into_iter()
                    .map(|item| {
                        let owner = hex::decode(item).map_err(Error::new)?;
                        PublicKey::from_bytes(&owner).map_err(Error::new)
                    })
                    .collect::<Result<Vec<PublicKey>>>()?;
                owners.push(public_key);

                nekoton::core::ton_wallet::multisig::prepare_deploy(
                    &SimpleClock,
                    &public_key,
                    DEFAULT_MULTISIG_TYPE,
                    address.workchain_id as i8,
                    Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT),
                    &owners,
                    address.confirmations.unwrap_or(1) as u8,
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
        let original_outputs = serde_json::to_value(transaction.outputs.clone())?;

        let bounce = transaction.bounce.unwrap_or_default();

        let public_key = PublicKey::from_bytes(public_key)?;
        let address = nekoton_utils::repack_address(&transaction.from_address.0)?;

        let expiration = Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT);

        let transfer_action = match account_type {
            AccountType::HighloadWallet => {
                let account = UInt256::from_be_bytes(&address.address().get_bytestring(0));
                let current_state = self.ton_core.get_contract_state(&account)?.account;

                let mut gifts: Vec<nekoton::core::ton_wallet::highload_wallet_v2::Gift> = vec![];
                for item in transaction.outputs {
                    let flags = item.output_type.unwrap_or_default();
                    let destination = nekoton_utils::repack_address(&item.recipient_address.0)?;
                    let amount = item.value.to_u64().ok_or(TonClientError::ParseBigDecimal)?;

                    gifts.push(nekoton::core::ton_wallet::highload_wallet_v2::Gift {
                        flags: flags.into(),
                        bounce,
                        destination,
                        amount,
                        body: None,
                        state_init: None,
                    });
                }

                nekoton::core::ton_wallet::highload_wallet_v2::prepare_transfer(
                    &SimpleClock,
                    &public_key,
                    &current_state,
                    gifts,
                    expiration,
                )?
            }
            AccountType::Wallet => {
                let account = UInt256::from_be_bytes(&address.address().get_bytestring(0));
                let current_state = self.ton_core.get_contract_state(&account)?.account;

                let recipient = transaction
                    .outputs
                    .first()
                    .ok_or(TonClientError::RecipientNotFound)?;

                let destination = nekoton_utils::repack_address(&recipient.recipient_address.0)?;
                let amount = recipient
                    .value
                    .to_u64()
                    .ok_or(TonClientError::ParseBigDecimal)?;
                let flags = recipient.output_type.clone().unwrap_or_default();

                nekoton::core::ton_wallet::wallet_v3::prepare_transfer(
                    &SimpleClock,
                    &public_key,
                    &current_state,
                    destination,
                    amount,
                    flags.into(),
                    bounce,
                    None,
                    expiration,
                )?
            }
            AccountType::SafeMultisig => {
                let recipient = transaction
                    .outputs
                    .first()
                    .ok_or(TonClientError::RecipientNotFound)?;

                let destination = nekoton_utils::repack_address(&recipient.recipient_address.0)?;
                let amount = recipient
                    .value
                    .to_u64()
                    .ok_or(TonClientError::ParseBigDecimal)?;
                let flags = recipient.output_type.clone().unwrap_or_default();

                let has_multiple_owners = match custodians {
                    Some(custodians) => *custodians > 1,
                    None => return Err(TonClientError::CustodiansNotFound.into()),
                };

                nekoton::core::ton_wallet::multisig::prepare_transfer(
                    &SimpleClock,
                    &public_key,
                    has_multiple_owners,
                    address.clone(),
                    destination,
                    amount,
                    flags.into(),
                    bounce,
                    None,
                    expiration,
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
    async fn prepare_confirm_transaction(
        &self,
        transaction: TransactionConfirm,
        public_key: &[u8],
        private_key: &[u8],
    ) -> Result<(SentTransaction, SignedMessage)> {
        let public_key = PublicKey::from_bytes(public_key)?;
        let address = nekoton_utils::repack_address(&transaction.address.0)?;

        let account_workchain_id = address.workchain_id();
        let account_hex = address.address().to_hex_string();

        let unsigned_message = nekoton::core::ton_wallet::multisig::prepare_confirm_transaction(
            &SimpleClock,
            &public_key,
            address,
            transaction.transaction_id,
            Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT),
        )?;

        let key_pair = Keypair {
            secret: SecretKey::from_bytes(private_key)?,
            public: public_key,
        };

        let signature = key_pair.sign(unsigned_message.hash());
        let signed_message = unsigned_message.sign(&signature.to_bytes())?;

        let sent_transaction = SentTransaction {
            id: transaction.id,
            message_hash: signed_message.message.hash()?.to_hex_string(),
            account_workchain_id,
            account_hex,
            original_value: None,
            original_outputs: None,
            aborted: false,
            bounce: false,
        };

        Ok((sent_transaction, signed_message))
    }
    async fn get_token_address_info(
        &self,
        owner: &MsgAddressInt,
        root_address: &MsgAddressInt,
    ) -> Result<NetworkTokenAddressData> {
        let root_account = UInt256::from_be_bytes(&root_address.address().get_bytestring(0));
        let root_contract = self.ton_core.get_contract_state(&root_account)?;

        let token_address = get_token_wallet_address(&root_contract, owner)?;
        let token_account = UInt256::from_be_bytes(&token_address.address().get_bytestring(0));
        let token_contract = match self.ton_core.get_contract_state(&token_account) {
            Ok(contract) => contract,
            Err(_) => {
                return Ok(NetworkTokenAddressData::uninit(
                    &token_address,
                    root_address,
                ))
            }
        };

        let (version, network_balance) = get_token_wallet_basic_info(&token_contract)?;
        let sync_u_time = token_contract.timings.current_utime(&SimpleClock) as i64;

        let (last_transaction_hash, last_transaction_lt) =
            parse_last_transaction(&token_contract.last_transaction_id);

        Ok(NetworkTokenAddressData {
            workchain_id: token_address.workchain_id(),
            hex: token_address.address().to_hex_string(),
            root_address: root_address.to_string(),
            version: version.to_string(),
            network_balance,
            account_status: token_contract.account.storage.state.into(),
            last_transaction_hash,
            last_transaction_lt,
            sync_u_time,
        })
    }
    async fn prepare_token_transaction(
        &self,
        input: &TokenTransactionSend,
        public_key: &[u8],
        private_key: &[u8],
        account_type: &AccountType,
        custodians: &Option<i32>,
    ) -> Result<(SentTransaction, SignedMessage)> {
        let owner = repack_address(&input.from_address.0)?;

        let token_owner_db = self
            .sqlx_client
            .get_token_address(
                owner.workchain_id(),
                owner.address().to_hex_string(),
                input.root_address.0.clone(),
            )
            .await?;
        let token_wallet = repack_address(&token_owner_db.address)?;

        let recipient = repack_address(&input.recipient_address.0)?;
        let destination = nekoton::core::models::TransferRecipient::OwnerWallet(recipient);

        let send_gas_to = match &input.send_gas_to {
            Some(send_gas_to) => repack_address(send_gas_to.0.as_str())?,
            None => owner.clone(),
        };

        let version = token_owner_db.version.into();

        let (value, _) = input.value.clone().as_bigint_and_exponent();
        let tokens = value.to_biguint().ok_or(TonClientError::ParseBigUint)?;

        let attached_amount = input.fee.to_u64().ok_or(TonClientError::ParseBigDecimal)?;

        let internal_message = prepare_token_transfer(
            owner.clone(),
            token_wallet,
            version,
            destination,
            tokens,
            send_gas_to,
            input.notify_receiver,
            attached_amount,
            Default::default(),
        )?;

        build_token_transaction(
            &self.ton_core,
            input.id,
            owner,
            public_key,
            private_key,
            account_type,
            custodians,
            internal_message,
        )
    }
    async fn prepare_token_burn(
        &self,
        input: &TokenTransactionBurn,
        public_key: &[u8],
        private_key: &[u8],
        account_type: &AccountType,
        custodians: &Option<i32>,
    ) -> Result<(SentTransaction, SignedMessage)> {
        let owner = repack_address(&input.from_address.0)?;

        let token_owner_db = self
            .sqlx_client
            .get_token_address(
                owner.workchain_id(),
                owner.address().to_hex_string(),
                input.root_address.0.clone(),
            )
            .await?;
        let token_wallet = repack_address(&token_owner_db.address)?;

        let send_gas_to = match &input.send_gas_to {
            Some(send_gas_to) => repack_address(send_gas_to.0.as_str())?,
            None => owner.clone(),
        };

        let callback_to = repack_address(input.callback_to.0.as_str())?;

        let version = token_owner_db.version.into();

        let (value, _) = input.value.clone().as_bigint_and_exponent();
        let tokens = value.to_biguint().ok_or(TonClientError::ParseBigUint)?;

        let attached_amount = input.fee.to_u64().ok_or(TonClientError::ParseBigDecimal)?;

        let internal_message = prepare_token_burn(
            owner.clone(),
            token_wallet,
            version,
            tokens,
            send_gas_to,
            callback_to,
            attached_amount,
            Default::default(),
        )?;

        build_token_transaction(
            &self.ton_core,
            input.id,
            owner,
            public_key,
            private_key,
            account_type,
            custodians,
            internal_message,
        )
    }
    async fn prepare_token_mint(
        &self,
        input: &TokenTransactionMint,
        public_key: &[u8],
        private_key: &[u8],
        account_type: &AccountType,
        custodians: &Option<i32>,
    ) -> Result<(SentTransaction, SignedMessage)> {
        let owner = repack_address(&input.owner_address.0)?;
        let root_token = repack_address(&input.root_address.0)?;
        let recipient = repack_address(&input.recipient_address.0)?;

        let root_account = UInt256::from_be_bytes(&root_token.address().get_bytestring(0));
        let root_contract = self.ton_core.get_contract_state(&root_account)?;

        let version = get_root_token_version(&root_contract)?;

        let (value, _) = input.value.clone().as_bigint_and_exponent();
        let tokens = value.to_biguint().ok_or(TonClientError::ParseBigUint)?;

        let deploy_wallet_value = BigUint::from_u64(
            input
                .deploy_wallet_value
                .to_u64()
                .ok_or(TonClientError::ParseBigDecimal)?,
        )
        .ok_or(TonClientError::ParseBigUint)?;

        let send_gas_to = match &input.send_gas_to {
            Some(send_gas_to) => repack_address(send_gas_to.0.as_str())?,
            None => owner.clone(),
        };

        let attached_amount = input.fee.to_u64().ok_or(TonClientError::ParseBigDecimal)?;

        let internal_message = prepare_token_mint(
            owner.clone(),
            root_token,
            version,
            tokens,
            recipient,
            deploy_wallet_value,
            send_gas_to,
            input.notify,
            attached_amount,
            Default::default(),
        )?;

        build_token_transaction(
            &self.ton_core,
            input.id,
            owner,
            public_key,
            private_key,
            account_type,
            custodians,
            internal_message,
        )
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
    fn add_pending_message(
        &self,
        account: UInt256,
        message_hash: UInt256,
        expire_at: u32,
    ) -> Result<oneshot::Receiver<MessageStatus>> {
        self.ton_core
            .add_pending_message(account, message_hash, expire_at)
    }
    async fn get_metrics(&self) -> Result<Metrics> {
        let gen_utime = self.ton_core.current_utime();
        Ok(Metrics { gen_utime })
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
    #[error("Parse BigDecimal error")]
    ParseBigDecimal,
    #[error("Parse BigUint error")]
    ParseBigUint,
}

fn build_token_transaction(
    ton_core: &Arc<TonCore>,
    id: Uuid,
    owner: MsgAddressInt,
    public_key: &[u8],
    private_key: &[u8],
    account_type: &AccountType,
    custodians: &Option<i32>,
    internal_message: InternalMessage,
) -> Result<(SentTransaction, SignedMessage)> {
    let flags = TransactionSendOutputType::default();

    let bounce = internal_message.bounce;
    let destination = internal_message.destination;
    let amount = internal_message.amount;
    let body = Some(internal_message.body);

    let expiration = Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT);

    let public_key = PublicKey::from_bytes(public_key).unwrap_or_default();

    let transfer_action = match account_type {
        AccountType::HighloadWallet => {
            let account = UInt256::from_be_bytes(&owner.address().get_bytestring(0));
            let current_state = ton_core.get_contract_state(&account)?.account;

            let gift = nekoton::core::ton_wallet::highload_wallet_v2::Gift {
                flags: flags.into(),
                bounce,
                destination,
                amount,
                body,
                state_init: None,
            };

            nekoton::core::ton_wallet::highload_wallet_v2::prepare_transfer(
                &SimpleClock,
                &public_key,
                &current_state,
                vec![gift],
                expiration,
            )?
        }
        AccountType::Wallet => {
            let account = UInt256::from_be_bytes(&owner.address().get_bytestring(0));
            let current_state = ton_core.get_contract_state(&account)?.account;

            nekoton::core::ton_wallet::wallet_v3::prepare_transfer(
                &SimpleClock,
                &public_key,
                &current_state,
                destination,
                amount,
                flags.into(),
                bounce,
                body,
                expiration,
            )?
        }
        AccountType::SafeMultisig => {
            let has_multiple_owners = match custodians {
                Some(custodians) => *custodians > 1,
                None => return Err(TonClientError::CustodiansNotFound.into()),
            };

            nekoton::core::ton_wallet::multisig::prepare_transfer(
                &SimpleClock,
                &public_key,
                has_multiple_owners,
                owner.clone(),
                destination,
                amount,
                flags.into(),
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
        bounce,
    };

    Ok((sent_transaction, signed_message))
}
