use std::str::FromStr;
use std::sync::Arc;

use axum::http::StatusCode;
use bigdecimal::{BigDecimal, ToPrimitive};
use ed25519_dalek::VerifyingKey;
use nekoton_core::contracts::blockchain_context::BlockchainContextBuilder;
use nekoton_core::contracts::function_ext::ExecutionOutput;
use nekoton_core::contracts::function_ext::FunctionExt;
use num_bigint::BigUint;
use num_traits::FromPrimitive;
use tokio::sync::oneshot;
use tycho_types::abi::{Function, NamedAbiValue, UnsignedExternalMessage};
use tycho_types::boc::Boc;
use tycho_types::cell::{CellBuilder, HashBytes};
use tycho_types::models::StdAddrFormat;
use tycho_types::models::{GlobalCapabilities, OwnedMessage, SignatureContext, StdAddr};
use tycho_util::time::now_sec;
use uuid::Uuid;

use crate::api::*;
use crate::models::*;
use crate::prelude::*;
use crate::services::*;
use crate::sqlx_client::*;
use crate::ton_core::*;
use crate::utils::mnemonic::{derive_from_phrase, generate_key, Bip39MnemonicData, MnemonicType};
use crate::utils::ton_wallet::multisig::DeployParams;
use crate::utils::ton_wallet::MultisigType;
use crate::utils::*;

#[derive(Clone)]
pub struct TonClient {
    ton_core: Arc<TonCore>,
    sqlx_client: SqlxClient,
}

impl TonClient {
    pub fn new(ton_core: Arc<TonCore>, sqlx_client: SqlxClient) -> Self {
        Self {
            ton_core,
            sqlx_client,
        }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        let owner_addresses = self
            .sqlx_client
            .get_all_addresses()
            .await?
            .into_iter()
            .map(|item| {
                StdAddr::from_str(&format!("{}:{}", item.workchain_id, item.hex))
                    .map_err(From::from)
            })
            .collect::<anyhow::Result<Vec<StdAddr>>>()?;

        // Subscribe to ton accounts
        let owner_accounts = owner_addresses
            .iter()
            .map(|item| item.address)
            .collect::<Vec<HashBytes>>();

        self.ton_core.add_ton_account_subscription(owner_accounts);

        Ok(())
    }

    pub async fn create_address(&self, payload: CreateAddress) -> Result<CreatedAddress, Error> {
        let generated_key = generate_key(MnemonicType::Bip39(Bip39MnemonicData::labs_old(0)));

        let signing_key =
            derive_from_phrase(&generated_key.words.join(" "), generated_key.account_type)?;

        let public = signing_key.verifying_key();

        let workchain_id = payload.workchain_id.unwrap_or_default();
        let account_type = payload.account_type.unwrap_or_default();

        let address = match account_type {
            AccountType::HighloadWallet => {
                ton_wallet::highload_wallet_v2::compute_contract_address(
                    &public,
                    workchain_id as i8,
                )
            }
            AccountType::Wallet => {
                ton_wallet::wallet_v3::compute_contract_address(&public, workchain_id as i8)
            }
            AccountType::SafeMultisig => ton_wallet::multisig::compute_contract_address(
                &public,
                MultisigType::SafeMultisigWallet,
                workchain_id as i8,
            ),
            AccountType::EverWallet => {
                ton_wallet::ever_wallet::compute_contract_address(&public, workchain_id as i8)
            }
        }?;

        let (custodians, confirmations) = match account_type {
            AccountType::SafeMultisig => (
                Some(payload.custodians.unwrap_or(1)),
                Some(payload.confirmations.unwrap_or(1)),
            ),
            AccountType::HighloadWallet | AccountType::Wallet | AccountType::EverWallet => {
                (None, None)
            }
        };

        if let (Some(custodians), Some(confirmations)) = (custodians, confirmations) {
            if confirmations > custodians {
                return Err(TonServiceError::WrongInput(
                    "Invalid number of confirmations".to_string(),
                )
                .into());
            }
        }

        // Validate custodians and append created pubkey to them
        let custodians_public_keys = match account_type {
            AccountType::SafeMultisig => {
                let public_keys = &payload.custodians_public_keys.unwrap_or_default();

                let mut custodians = Vec::with_capacity(public_keys.len());
                for key in public_keys {
                    let decoded_key = hex::decode(key).map_err(|_| {
                        TonServiceError::WrongInput("Invalid custodian".to_string())
                    })?;
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&decoded_key);

                    custodians.push(VerifyingKey::from_bytes(&key).map_err(|_| {
                        TonServiceError::WrongInput("Invalid custodian".to_string())
                    })?);
                }
                custodians.push(public);

                let custodians = custodians
                    .into_iter()
                    .map(|key| hex::encode(key.to_bytes()))
                    .collect();

                Some(custodians)
            }
            AccountType::HighloadWallet | AccountType::Wallet | AccountType::EverWallet => None,
        };

        // Subscribe to accounts
        let account = address.address;
        self.ton_core.add_ton_account_subscription([account]);

        Ok(CreatedAddress {
            workchain_id: address.workchain as i32,
            hex: address.address.to_string(),
            base64url: address.display_base64_url(true).to_string(),
            public_key: public.to_bytes().to_vec(),
            private_key: signing_key.to_bytes().to_vec(),
            account_type,
            custodians,
            confirmations,
            custodians_public_keys,
        })
    }

    pub async fn get_address_info(&self, owner: &StdAddr) -> Result<NetworkAddressData, Error> {
        let account = owner.address;
        let contract = match self.ton_core.get_contract_state(&account) {
            Ok(contract) => contract,
            Err(_) => return Ok(NetworkAddressData::uninit(owner)),
        };

        let network_balance = BigDecimal::from_u128(contract.account.balance.tokens.into_inner())
            .ok_or(TonClientError::ParseBigDecimal)?;

        Ok(NetworkAddressData {
            workchain_id: contract.account.address.workchain(),
            hex: contract.account.address.to_string(),
            account_status: contract.account.state.into(),
            network_balance,
            last_transaction_hash: Some(contract.last_transaction_hash.to_string()),
            last_transaction_lt: Some(contract.account.last_trans_lt.to_string()),
            sync_u_time: 0i64, // TODO fix
        })
    }

    pub async fn prepare_deploy(
        &self,
        address: &AddressDb,
        public_key: &[u8],
        private_key: &[u8],
    ) -> Result<Option<PrepareResult>, Error> {
        let mut key = [0u8; 32];
        key.copy_from_slice(public_key);

        let public_key = VerifyingKey::from_bytes(&key)?;
        let expire_at = now_sec() + DEFAULT_EXPIRATION_TIMEOUT;

        let unsigned_message = match address.account_type {
            AccountType::SafeMultisig => {
                let custodians: Vec<String> = serde_json::from_value(
                    address.custodians_public_keys.clone().unwrap_or_default(),
                )?;

                let owners = custodians
                    .into_iter()
                    .map(|item| {
                        let mut key = [0u8; 32];
                        key.copy_from_slice(&hex::decode(item).map_err(anyhow::Error::from)?);
                        VerifyingKey::from_bytes(&key).map_err(anyhow::Error::from)
                    })
                    .collect::<Result<Vec<VerifyingKey>, anyhow::Error>>()?;

                ton_wallet::multisig::prepare_deploy(
                    &public_key,
                    MultisigType::SafeMultisigWallet,
                    address.workchain_id as i8,
                    expire_at,
                    DeployParams {
                        owners: &owners,
                        req_confirms: address.confirmations.unwrap_or_default() as u8,
                        expiration_time: None,
                    },
                )?
            }
            AccountType::EverWallet => ton_wallet::ever_wallet::prepare_deploy(
                &public_key,
                address.workchain_id as i8,
                expire_at,
            )?,
            AccountType::HighloadWallet | AccountType::Wallet => {
                return Ok(None);
            }
        };

        let mut key = [0u8; 32];
        key.copy_from_slice(private_key);

        let key_pair = ed25519_dalek::SigningKey::from_bytes(&key);

        let context = SignatureContext {
            global_id: self.ton_core.signature_id().unwrap_or_default(),
            capabilities: GlobalCapabilities::new(self.ton_core.capabilities()),
        };

        let owned_message = unsigned_message.sign(&key_pair, context)?;

        let cell_builder = CellBuilder::build_from(&owned_message).map_err(anyhow::Error::from)?;
        let hash = cell_builder.repr_hash();

        let sent_transaction = SentTransaction {
            id: Uuid::new_v4(),
            message_hash: hash.to_string(),
            account_workchain_id: address.workchain_id,
            account_hex: address.hex.clone(),
            original_value: None,
            original_outputs: None,
            aborted: false,
            bounce: false,
        };

        Ok(Some(PrepareResult {
            sent_transaction,
            owned_message,
            expire_at,
        }))
    }

    pub async fn prepare_transaction(
        &self,
        transaction: TransactionSend,
        public_key: &[u8],
        private_key: &[u8],
        account_type: &AccountType,
        custodians: &Option<i32>,
    ) -> Result<PrepareResult, Error> {
        let original_value = transaction.outputs.iter().map(|o| o.value.clone()).sum();
        let original_outputs = serde_json::to_value(transaction.outputs.clone())?;
        let bounce = transaction.bounce.unwrap_or_default();

        let mut key = [0u8; 32];
        key.copy_from_slice(public_key);

        let public_key = VerifyingKey::from_bytes(&key)?;

        let (address, _) = StdAddr::from_str_ext(&transaction.from_address.0, StdAddrFormat::any())
            .map_err(anyhow::Error::from)?;

        let expire_at = now_sec() + DEFAULT_EXPIRATION_TIMEOUT;

        // parse input payload
        let body = transaction
            .payload
            .map(Boc::decode_base64)
            .transpose()
            .map_err(anyhow::Error::from)?;

        let unsigned_message = match account_type {
            AccountType::HighloadWallet => {
                let account = address.address;
                let current_state = self.ton_core.get_contract_state(&account)?.account;

                let mut gifts: Vec<ton_wallet::Gift> = vec![];
                for item in transaction.outputs {
                    let flags = item.output_type.unwrap_or_default();
                    let (destination, _) =
                        StdAddr::from_str_ext(&item.recipient_address.0, StdAddrFormat::any())
                            .map_err(anyhow::Error::from)?;
                    let amount = item
                        .value
                        .to_u128()
                        .ok_or(TonClientError::ParseBigDecimal)?;

                    gifts.push(ton_wallet::Gift {
                        flags: flags.into(),
                        bounce,
                        destination,
                        amount,
                        body: body.clone(),
                        state_init: None,
                    });
                }
                ton_wallet::highload_wallet_v2::prepare_transfer(
                    &public_key,
                    &current_state,
                    gifts,
                    expire_at,
                )?
            }
            AccountType::Wallet => {
                let account = address.address;
                let current_state = self.ton_core.get_contract_state(&account)?.account;

                let recipient = transaction
                    .outputs
                    .first()
                    .ok_or(TonClientError::RecipientNotFound)?;
                let (destination, _) =
                    StdAddr::from_str_ext(&recipient.recipient_address.0, StdAddrFormat::any())
                        .map_err(anyhow::Error::from)?;

                let amount = recipient
                    .value
                    .to_u128()
                    .ok_or(TonClientError::ParseBigDecimal)?;
                let flags = recipient.output_type.clone().unwrap_or_default();

                let gifts = vec![ton_wallet::Gift {
                    flags: flags.into(),
                    bounce,
                    destination,
                    amount,
                    body,
                    state_init: None,
                }];
                let seqno_offset =
                    ton_wallet::wallet_v3::estimate_seqno_offset(&current_state, &[]);
                ton_wallet::wallet_v3::prepare_transfer(
                    &public_key,
                    &current_state,
                    seqno_offset,
                    gifts,
                    expire_at,
                )?
            }
            AccountType::SafeMultisig => {
                let recipient = transaction
                    .outputs
                    .first()
                    .ok_or(TonClientError::RecipientNotFound)?;
                let (destination, _) =
                    StdAddr::from_str_ext(&recipient.recipient_address.0, StdAddrFormat::any())
                        .map_err(anyhow::Error::from)?;
                let amount = recipient
                    .value
                    .to_u128()
                    .ok_or(TonClientError::ParseBigDecimal)?;
                let flags = recipient.output_type.clone().unwrap_or_default();
                let has_multiple_owners = match custodians {
                    Some(custodians) => *custodians > 1,
                    None => return Err(TonClientError::CustodiansNotFound.into()),
                };

                let gift = ton_wallet::Gift {
                    flags: flags.into(),
                    bounce,
                    destination,
                    amount,
                    body,
                    state_init: None,
                };
                ton_wallet::multisig::prepare_transfer(
                    MultisigType::SafeMultisigWallet,
                    &public_key,
                    has_multiple_owners,
                    address.clone(),
                    gift,
                    expire_at,
                )?
            }
            AccountType::EverWallet => {
                let account = address.address;
                let current_state = self.ton_core.get_contract_state(&account)?.account;

                let mut gifts: Vec<ton_wallet::Gift> = vec![];
                for item in transaction.outputs {
                    let flags = item.output_type.unwrap_or_default();
                    let (destination, _) =
                        StdAddr::from_str_ext(&item.recipient_address.0, StdAddrFormat::any())
                            .map_err(anyhow::Error::from)?;
                    let amount = item
                        .value
                        .to_u128()
                        .ok_or(TonClientError::ParseBigDecimal)?;
                    gifts.push(ton_wallet::Gift {
                        flags: flags.into(),
                        bounce,
                        destination,
                        amount,
                        body: body.clone(),
                        state_init: None,
                    });
                }
                ton_wallet::ever_wallet::prepare_transfer(
                    &public_key,
                    &current_state,
                    address.clone(),
                    gifts,
                    expire_at,
                )?
            }
        };

        let mut key = [0u8; 32];
        key.copy_from_slice(private_key);

        let key_pair = ed25519_dalek::SigningKey::from_bytes(&key);

        let context = SignatureContext {
            global_id: self.ton_core.signature_id().unwrap_or_default(),
            capabilities: GlobalCapabilities::new(self.ton_core.capabilities()),
        };

        let owned_message = unsigned_message.sign(&key_pair, context)?;

        let cell_builder = CellBuilder::build_from(&owned_message).map_err(anyhow::Error::from)?;
        let message_hash = cell_builder.repr_hash();

        let sent_transaction = SentTransaction {
            id: transaction.id,
            message_hash: message_hash.to_string(),
            account_workchain_id: address.workchain as i32,
            account_hex: address.address.to_string(),
            original_value: Some(original_value),
            original_outputs: Some(original_outputs),
            aborted: false,
            bounce,
        };
        Ok(PrepareResult {
            sent_transaction,
            owned_message,
            expire_at,
        })
    }

    pub async fn prepare_confirm_transaction(
        &self,
        transaction: TransactionConfirm,
        public_key: &[u8],
        private_key: &[u8],
    ) -> Result<PrepareResult, Error> {
        let mut key = [0u8; 32];
        key.copy_from_slice(public_key);

        let public_key = VerifyingKey::from_bytes(&key)?;

        let (address, _) = StdAddr::from_str_ext(&transaction.address.0, StdAddrFormat::any())
            .map_err(anyhow::Error::from)?;

        let account_workchain_id = address.workchain as i32;
        let account_hex = address.address.to_string();

        let expire_at = now_sec() + DEFAULT_EXPIRATION_TIMEOUT;

        let unsigned_message = ton_wallet::multisig::prepare_confirm_transaction(
            MultisigType::SafeMultisigWallet,
            &public_key,
            address,
            transaction.transaction_id,
            expire_at,
        )?;

        let mut key = [0u8; 32];
        key.copy_from_slice(private_key);

        let key_pair = ed25519_dalek::SigningKey::from_bytes(&key);

        let context = SignatureContext {
            global_id: self.ton_core.signature_id().unwrap_or_default(),
            capabilities: GlobalCapabilities::new(self.ton_core.capabilities()),
        };

        let owned_message = unsigned_message.sign(&key_pair, context)?;

        let cell_builder = CellBuilder::build_from(&owned_message).map_err(anyhow::Error::from)?;
        let message_hash = cell_builder.repr_hash();

        let sent_transaction = SentTransaction {
            id: transaction.id,
            message_hash: message_hash.to_string(),
            account_workchain_id,
            account_hex,
            original_value: None,
            original_outputs: None,
            aborted: false,
            bounce: false,
        };

        Ok(PrepareResult {
            sent_transaction,
            owned_message,
            expire_at,
        })
    }

    pub async fn get_token_address_info(
        &self,
        owner: &StdAddr,
        root_address: &StdAddr,
    ) -> Result<NetworkTokenAddressData, Error> {
        let root_account = root_address.address;
        let root_contract = self.ton_core.get_contract_state(&root_account)?;

        let context = BlockchainContextBuilder::new().build()?;

        let token_address = get_token_wallet_address(root_contract, context.clone(), owner)?;
        let token_account = token_address.address;
        let token_contract = match self.ton_core.get_contract_state(&token_account) {
            Ok(contract) => contract,
            Err(_) => {
                return Ok(NetworkTokenAddressData::uninit(
                    &token_address,
                    root_address,
                ))
            }
        };

        let account_status = token_contract.account.state.clone().into();
        let last_transaction_hash = Some(token_contract.last_transaction_hash.to_string());
        let last_transaction_lt = Some(token_contract.account.last_trans_lt.to_string());

        let (version, network_balance) = get_token_wallet_basic_info(token_contract, context)?;

        Ok(NetworkTokenAddressData {
            workchain_id: token_address.workchain as i32,
            hex: token_address.address.to_string(),
            root_address: root_address.to_string(),
            version: version.to_string(),
            network_balance,
            account_status,
            last_transaction_hash,
            last_transaction_lt,
            sync_u_time: 0, // TODO: fix
        })
    }

    pub async fn prepare_token_transaction(
        &self,
        input: &TokenTransactionSend,
        public_key: &[u8],
        private_key: &[u8],
        account_type: &AccountType,
        custodians: &Option<i32>,
    ) -> Result<PrepareResult, Error> {
        let (owner, _) = StdAddr::from_str_ext(&input.from_address.0, StdAddrFormat::any())
            .map_err(anyhow::Error::from)?;

        let token_owner_db = self
            .sqlx_client
            .get_token_address(
                owner.workchain as i32,
                owner.address.to_string(),
                input.root_address.0.clone(),
            )
            .await?;
        let token_wallet =
            StdAddr::from_str(&token_owner_db.address).map_err(anyhow::Error::from)?;

        let (destination, _) =
            StdAddr::from_str_ext(&input.recipient_address.0, StdAddrFormat::any())
                .map_err(anyhow::Error::from)?;

        let send_gas_to = match &input.send_gas_to {
            Some(send_gas_to) => {
                let (send_gas_to, _) = StdAddr::from_str_ext(&send_gas_to.0, StdAddrFormat::any())
                    .map_err(anyhow::Error::from)?;
                send_gas_to
            }
            None => owner.clone(),
        };

        let version = token_owner_db.version.into();

        let (value, _) = input.value.clone().as_bigint_and_exponent();
        let tokens = value.to_biguint().ok_or(TonClientError::ParseBigUint)?;

        let attached_amount = input.fee.to_u128().ok_or(TonClientError::ParseBigDecimal)?;

        // parse input payload

        let body = input
            .payload
            .as_ref()
            .map(Boc::decode_base64)
            .transpose()
            .map_err(anyhow::Error::from)?;

        let internal_message = prepare_token_transfer(
            owner.clone(),
            token_wallet,
            version,
            destination,
            tokens,
            send_gas_to,
            input.notify_receiver,
            attached_amount,
            body.unwrap_or_default(),
        )?;

        let res = build_token_transaction(
            &self.ton_core,
            input.id,
            owner,
            public_key,
            private_key,
            account_type,
            custodians,
            internal_message,
        )?;

        Ok(res)
    }

    pub async fn prepare_token_burn(
        &self,
        input: &TokenTransactionBurn,
        public_key: &[u8],
        private_key: &[u8],
        account_type: &AccountType,
        custodians: &Option<i32>,
    ) -> Result<PrepareResult, Error> {
        let (owner, _) = StdAddr::from_str_ext(&input.from_address.0, StdAddrFormat::any())
            .map_err(anyhow::Error::from)?;

        let token_owner_db = self
            .sqlx_client
            .get_token_address(
                owner.workchain as i32,
                owner.address.to_string(),
                input.root_address.0.clone(),
            )
            .await?;

        let token_wallet =
            StdAddr::from_str(&token_owner_db.address).map_err(anyhow::Error::from)?;

        let send_gas_to = match &input.send_gas_to {
            Some(send_gas_to) => {
                let (send_gas_to, _) = StdAddr::from_str_ext(&send_gas_to.0, StdAddrFormat::any())
                    .map_err(anyhow::Error::from)?;
                send_gas_to
            }
            None => owner.clone(),
        };

        let (callback_to, _) = StdAddr::from_str_ext(&input.callback_to.0, StdAddrFormat::any())
            .map_err(anyhow::Error::from)?;

        let version = token_owner_db.version.into();

        let (value, _) = input.value.clone().as_bigint_and_exponent();
        let tokens = value.to_biguint().ok_or(TonClientError::ParseBigUint)?;

        let attached_amount = input.fee.to_u128().ok_or(TonClientError::ParseBigDecimal)?;

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

        let res = build_token_transaction(
            &self.ton_core,
            input.id,
            owner,
            public_key,
            private_key,
            account_type,
            custodians,
            internal_message,
        )?;

        Ok(res)
    }

    pub async fn prepare_token_mint(
        &self,
        input: &TokenTransactionMint,
        public_key: &[u8],
        private_key: &[u8],
        account_type: &AccountType,
        custodians: &Option<i32>,
    ) -> Result<PrepareResult, Error> {
        let (owner, _) = StdAddr::from_str_ext(&input.owner_address.0, StdAddrFormat::any())
            .map_err(anyhow::Error::from)?;
        let (root_token, _) = StdAddr::from_str_ext(&input.root_address.0, StdAddrFormat::any())
            .map_err(anyhow::Error::from)?;
        let (recipient, _) =
            StdAddr::from_str_ext(&input.recipient_address.0, StdAddrFormat::any())
                .map_err(anyhow::Error::from)?;

        let root_account = root_token.address;
        let root_contract = self.ton_core.get_contract_state(&root_account)?;
        let context = BlockchainContextBuilder::new().build()?;

        let version = get_root_token_version(root_contract, context)?;

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
            Some(send_gas_to) => {
                let (send_gas_to, _) = StdAddr::from_str_ext(&send_gas_to.0, StdAddrFormat::any())
                    .map_err(anyhow::Error::from)?;
                send_gas_to
            }
            None => owner.clone(),
        };

        let attached_amount = input.fee.to_u128().ok_or(TonClientError::ParseBigDecimal)?;

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

        let res = build_token_transaction(
            &self.ton_core,
            input.id,
            owner,
            public_key,
            private_key,
            account_type,
            custodians,
            internal_message,
        )?;

        Ok(res)
    }

    pub async fn send_transaction(
        &self,
        account: HashBytes,
        owned_message: OwnedMessage,
        expire_at: u32,
    ) -> Result<MessageStatus, Error> {
        let status = self
            .ton_core
            .send_ton_message(account, owned_message, expire_at)
            .await?;

        Ok(status)
    }

    pub fn add_pending_message(
        &self,
        account: HashBytes,
        message_hash: HashBytes,
        expire_at: u32,
    ) -> Result<oneshot::Receiver<MessageStatus>, Error> {
        let status = self
            .ton_core
            .add_pending_message(account, message_hash, expire_at)?;

        Ok(status)
    }

    pub async fn get_metrics(&self) -> Result<Metrics, Error> {
        let gen_utime = self.ton_core.current_utime();
        Ok(Metrics { gen_utime })
    }

    pub async fn get_blockchain_info(&self) -> Result<BlockchainInfo, Error> {
        let gen_utime = self.ton_core.current_utime();

        let network_id = match () {
            _ => TYCHO_TESTNET_CHAIN_ID,
        };

        let subscriber_metrics = self.ton_core.context.ton_subscriber.metrics();

        Ok(BlockchainInfo {
            network_id,
            synced: subscriber_metrics.ready,
            subscriber_pending_messages: subscriber_metrics.pending_message_count,
            tip_block_ts: gen_utime,
            masterchain_height: 0,
            masterchain_last_updated: 0,
        })
    }

    pub async fn run_local(
        &self,
        contract_address: HashBytes,
        function: Function,
        input: &[NamedAbiValue],
        responsible: bool,
    ) -> anyhow::Result<Option<Vec<NamedAbiValue>>> {
        let mut state = match self.ton_core.get_contract_state(&contract_address) {
            Ok(a) => a,
            Err(e) => {
                tracing::error!("Failed to get contract state: {e:?}");
                return Ok(None);
            }
        };

        let ExecutionOutput { values, exit_code } = function.run_local(
            &mut state.account,
            input,
            responsible,
            &mut self.ton_core.blockchain_context(),
        )?;

        if exit_code != 0 {
            return Err(anyhow::anyhow!("Non-zero result code: {exit_code}"));
        }

        Ok(Some(values))
    }

    pub async fn prepare_signed_generic_message(
        &self,
        sender_addr: &str,
        public_key: &[u8],
        private_key: &[u8],
        target_addr: &str,
        execution_flag: u8,
        value: BigDecimal,
        bounce: bool,
        account_type: &AccountType,
        custodians: &Option<i32>,
        function: Option<Function>,
        params: Option<Vec<NamedAbiValue>>,
    ) -> Result<(OwnedMessage, u32), Error> {
        let unsigned_message = self
            .prepare_generic_message(
                sender_addr,
                public_key,
                target_addr,
                execution_flag,
                value,
                bounce,
                account_type,
                custodians,
                function,
                params,
            )
            .await?;

        let mut key = [0u8; 32];
        key.copy_from_slice(private_key);

        let key_pair = ed25519_dalek::SigningKey::from_bytes(&key);
        let expire_at = unsigned_message.expire_at();

        let context = SignatureContext {
            global_id: self.ton_core.signature_id().unwrap_or_default(),
            capabilities: GlobalCapabilities::new(self.ton_core.capabilities()),
        };

        let owned_message = unsigned_message.sign(&key_pair, context)?;

        Ok((owned_message, expire_at))
    }

    pub async fn prepare_generic_message(
        &self,
        sender_addr: &str,
        public_key: &[u8],
        target_addr: &str,
        execution_flag: u8,
        value: BigDecimal,
        bounce: bool,
        account_type: &AccountType,
        custodians: &Option<i32>,
        function: Option<Function>,
        params: Option<Vec<NamedAbiValue>>,
    ) -> Result<UnsignedExternalMessage, Error> {
        let mut key = [0u8; 32];
        key.copy_from_slice(public_key);

        let public_key = VerifyingKey::from_bytes(&key)?;

        let (address, _) = StdAddr::from_str_ext(sender_addr, StdAddrFormat::any())
            .map_err(anyhow::Error::from)?;

        let expire_at = now_sec() + DEFAULT_EXPIRATION_TIMEOUT;

        let function_data = function.and_then(|function| {
            let tokens = params.unwrap_or_default();
            function.encode_internal_input(&tokens).ok()
        });
        let body = function_data
            .map(|data| data.build())
            .transpose()
            .map_err(anyhow::Error::from)?;

        let (destination, _) = StdAddr::from_str_ext(target_addr, StdAddrFormat::any())
            .map_err(anyhow::Error::from)?;

        let amount = value.to_u128().ok_or(TonClientError::ParseBigDecimal)?;
        let unsigned_message = match account_type {
            AccountType::Wallet => {
                let account = address.address;
                let current_state = self.ton_core.get_contract_state(&account)?.account;

                let gifts = vec![ton_wallet::Gift {
                    flags: execution_flag,
                    bounce,
                    destination,
                    amount,
                    body,
                    state_init: None,
                }];

                let seqno_offset =
                    ton_wallet::wallet_v3::estimate_seqno_offset(&current_state, &[]);

                ton_wallet::wallet_v3::prepare_transfer(
                    &public_key,
                    &current_state,
                    seqno_offset,
                    gifts,
                    expire_at,
                )?
            }
            AccountType::SafeMultisig => {
                let has_multiple_owners = match custodians {
                    Some(custodians) => *custodians > 1,
                    None => return Err(TonClientError::CustodiansNotFound.into()),
                };

                let gift = ton_wallet::Gift {
                    flags: execution_flag,
                    bounce,
                    destination,
                    amount,
                    body,
                    state_init: None,
                };

                ton_wallet::multisig::prepare_transfer(
                    MultisigType::SafeMultisigWallet,
                    &public_key,
                    has_multiple_owners,
                    address,
                    gift,
                    expire_at,
                )?
            }
            AccountType::HighloadWallet => {
                let account = address.address;
                let current_state = self.ton_core.get_contract_state(&account)?.account;

                let gift = ton_wallet::Gift {
                    flags: execution_flag,
                    bounce,
                    destination,
                    amount,
                    body,
                    state_init: None,
                };

                ton_wallet::highload_wallet_v2::prepare_transfer(
                    &public_key,
                    &current_state,
                    vec![gift],
                    expire_at,
                )?
            }
            AccountType::EverWallet => {
                let account = address.address;
                let current_state = self.ton_core.get_contract_state(&account)?.account;

                let gift = ton_wallet::Gift {
                    flags: execution_flag,
                    bounce,
                    destination,
                    amount,
                    body,
                    state_init: None,
                };

                ton_wallet::ever_wallet::prepare_transfer(
                    &public_key,
                    &current_state,
                    address,
                    vec![gift],
                    expire_at,
                )?
            }
        };

        Ok(unsigned_message)
    }

    pub fn add_ton_account_subscription(&self, account: HashBytes) {
        self.ton_core.add_ton_account_subscription([account])
    }

    pub fn add_ton_account_subscriptions<I>(&self, accounts: I)
    where
        I: Iterator<Item = HashBytes>,
    {
        self.ton_core.add_ton_account_subscription(accounts)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum TonClientError {
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

impl TonClientError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            TonClientError::ParseBigUint
            | TonClientError::RecipientNotFound
            | TonClientError::AccountNotDeployed(_) => StatusCode::BAD_REQUEST,
            TonClientError::CustodiansNotFound | TonClientError::ParseBigDecimal => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

fn build_token_transaction(
    ton_core: &Arc<TonCore>,
    id: Uuid,
    owner: StdAddr,
    public_key: &[u8],
    private_key: &[u8],
    account_type: &AccountType,
    custodians: &Option<i32>,
    internal_message: InternalMessage,
) -> anyhow::Result<PrepareResult> {
    let flags = TransactionSendOutputType::default();

    let bounce = internal_message.bounce;
    let destination = internal_message.destination;
    let amount = internal_message.amount;
    let body = Some(internal_message.body);

    let mut key = [0u8; 32];
    key.copy_from_slice(public_key);

    let public_key = VerifyingKey::from_bytes(&key)?;

    let expire_at = now_sec() + DEFAULT_EXPIRATION_TIMEOUT;

    let unsigned_message = match account_type {
        AccountType::HighloadWallet => {
            let account = owner.address;
            let current_state = ton_core.get_contract_state(&account)?.account;

            let gift = ton_wallet::Gift {
                flags: flags.into(),
                bounce,
                destination,
                amount,
                body,
                state_init: None,
            };

            ton_wallet::highload_wallet_v2::prepare_transfer(
                &public_key,
                &current_state,
                vec![gift],
                expire_at,
            )?
        }
        AccountType::Wallet => {
            let account = owner.address;
            let current_state = ton_core.get_contract_state(&account)?.account;

            let gifts = vec![ton_wallet::Gift {
                flags: flags.into(),
                bounce,
                destination,
                amount,
                body,
                state_init: None,
            }];

            let seqno_offset = ton_wallet::wallet_v3::estimate_seqno_offset(&current_state, &[]);

            ton_wallet::wallet_v3::prepare_transfer(
                &public_key,
                &current_state,
                seqno_offset,
                gifts,
                expire_at,
            )?
        }
        AccountType::SafeMultisig => {
            let has_multiple_owners = match custodians {
                Some(custodians) => *custodians > 1,
                None => return Err(TonClientError::CustodiansNotFound.into()),
            };

            let gift = ton_wallet::Gift {
                flags: flags.into(),
                bounce,
                destination,
                amount,
                body,
                state_init: None,
            };

            ton_wallet::multisig::prepare_transfer(
                MultisigType::SafeMultisigWallet,
                &public_key,
                has_multiple_owners,
                owner.clone(),
                gift,
                expire_at,
            )?
        }
        AccountType::EverWallet => {
            let account = owner.address;
            let current_state = ton_core.get_contract_state(&account)?.account;

            let gift = ton_wallet::Gift {
                flags: flags.into(),
                bounce,
                destination,
                amount,
                body,
                state_init: None,
            };

            ton_wallet::ever_wallet::prepare_transfer(
                &public_key,
                &current_state,
                owner.clone(),
                vec![gift],
                expire_at,
            )?
        }
    };

    let mut key = [0u8; 32];
    key.copy_from_slice(private_key);

    let key_pair = ed25519_dalek::SigningKey::from_bytes(&key);

    let context = SignatureContext {
        global_id: ton_core.signature_id().unwrap_or_default(),
        capabilities: GlobalCapabilities::new(ton_core.capabilities()),
    };

    let owned_message = unsigned_message.sign(&key_pair, context)?;

    let cell_builder = CellBuilder::build_from(&owned_message).map_err(anyhow::Error::from)?;
    let hash = cell_builder.repr_hash();

    let sent_transaction = SentTransaction {
        id,
        message_hash: hash.to_string(),
        account_workchain_id: owner.workchain as i32,
        account_hex: owner.address.to_string(),
        original_value: None,
        original_outputs: None,
        aborted: false,
        bounce,
    };

    Ok(PrepareResult {
        sent_transaction,
        owned_message,
        expire_at,
    })
}

const TYCHO_TESTNET_CHAIN_ID: i32 = -4000;

#[derive(Debug)]
pub struct PrepareResult {
    pub sent_transaction: SentTransaction,
    pub owned_message: OwnedMessage,
    pub expire_at: u32,
}
