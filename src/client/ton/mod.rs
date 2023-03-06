use std::sync::Arc;

use bigdecimal::{BigDecimal, ToPrimitive};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer};
use http::StatusCode;
use nekoton::core::models::Expiration;
use nekoton::core::ton_wallet::multisig::DeployParams;
use nekoton::core::ton_wallet::{MultisigType, TransferAction};
use nekoton::core::InternalMessage;
use nekoton::crypto::{SignedMessage, UnsignedMessage};
use nekoton_abi::MessageBuilder;
use nekoton_utils::{SimpleClock, TrustMe};
use num_bigint::BigUint;
use num_traits::FromPrimitive;
use tokio::sync::oneshot;
use ton_block::{GetRepresentationHash, MsgAddressInt};
use ton_types::{deserialize_tree_of_cells, UInt256};
use uuid::Uuid;

use crate::api::*;
use crate::models::*;
use crate::prelude::*;
use crate::services::*;
use crate::sqlx_client::*;
use crate::ton_core::*;
use crate::utils::*;

mod utils;

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

        Ok(())
    }

    pub async fn create_address(&self, payload: CreateAddress) -> Result<CreatedAddress, Error> {
        let generated_key = nekoton::crypto::generate_key(nekoton::crypto::MnemonicType::Labs(0));

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
                    MultisigType::SafeMultisigWallet,
                    workchain_id as i8,
                )
            }
        };

        let (custodians, confirmations) = match account_type {
            AccountType::SafeMultisig => (
                Some(payload.custodians.unwrap_or(1)),
                Some(payload.confirmations.unwrap_or(1)),
            ),
            AccountType::HighloadWallet | AccountType::Wallet => (None, None),
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
                    custodians.push(
                        PublicKey::from_bytes(&hex::decode(key).map_err(|_| {
                            TonServiceError::WrongInput("Invalid custodian".to_string())
                        })?)
                        .map_err(|_| {
                            TonServiceError::WrongInput("Invalid custodian".to_string())
                        })?,
                    );
                }
                custodians.push(public);

                let custodians = custodians
                    .into_iter()
                    .map(|key| hex::encode(key.to_bytes()))
                    .collect();

                Some(custodians)
            }
            AccountType::HighloadWallet | AccountType::Wallet => None,
        };

        // Subscribe to accounts
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

    pub async fn get_address_info(
        &self,
        owner: &MsgAddressInt,
    ) -> Result<NetworkAddressData, Error> {
        let account = UInt256::from_be_bytes(&owner.address().get_bytestring(0));
        let contract = match self.ton_core.get_contract_state(&account) {
            Ok(contract) => contract,
            Err(_) => return Ok(NetworkAddressData::uninit(owner)),
        };

        let network_balance = BigDecimal::from_u128(contract.account.storage.balance.grams.0)
            .ok_or(TonClientError::ParseBigDecimal)?;

        let (last_transaction_hash, last_transaction_lt) =
            utils::parse_last_transaction(&contract.last_transaction_id);

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

    pub async fn prepare_deploy(
        &self,
        address: &AddressDb,
        public_key: &[u8],
        private_key: &[u8],
    ) -> Result<Option<(SentTransaction, SignedMessage)>, Error> {
        let public_key = PublicKey::from_bytes(public_key)?;

        let unsigned_message = match address.account_type {
            AccountType::SafeMultisig => {
                let custodians: Vec<String> =
                    serde_json::from_value(address.custodians_public_keys.clone().trust_me())
                        .trust_me();

                let owners = custodians
                    .into_iter()
                    .map(|item| PublicKey::from_bytes(&hex::decode(item).trust_me()).trust_me())
                    .collect::<Vec<PublicKey>>();

                nekoton::core::ton_wallet::multisig::prepare_deploy(
                    &SimpleClock,
                    &public_key,
                    MultisigType::SafeMultisigWallet,
                    address.workchain_id as i8,
                    Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT),
                    DeployParams {
                        owners: &owners,
                        req_confirms: address.confirmations.trust_me() as u8,
                        expiration_time: None,
                    },
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

        let data_to_sign = ton_abi::extend_signature_with_id(
            unsigned_message.hash(),
            self.ton_core.signature_id(),
        );
        let signature = key_pair.sign(&data_to_sign);
        let signed_message = unsigned_message.sign(&signature.to_bytes())?;

        let sent_transaction = SentTransaction {
            id: Uuid::new_v4(),
            message_hash: signed_message.message.hash()?.to_hex_string(),
            account_workchain_id: address.workchain_id,
            account_hex: address.hex.clone(),
            original_value: None,
            original_outputs: None,
            aborted: false,
            bounce: false,
        };

        Ok(Some((sent_transaction, signed_message)))
    }

    pub async fn prepare_transaction(
        &self,
        transaction: TransactionSend,
        public_key: &[u8],
        private_key: &[u8],
        account_type: &AccountType,
        custodians: &Option<i32>,
    ) -> Result<(SentTransaction, SignedMessage), Error> {
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

                let mut gifts: Vec<nekoton::core::ton_wallet::Gift> = vec![];
                for item in transaction.outputs {
                    let flags = item.output_type.unwrap_or_default();
                    let destination = nekoton_utils::repack_address(&item.recipient_address.0)?;
                    let amount = item.value.to_u64().ok_or(TonClientError::ParseBigDecimal)?;

                    gifts.push(nekoton::core::ton_wallet::Gift {
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

                let gifts = vec![nekoton::core::ton_wallet::Gift {
                    flags: flags.into(),
                    bounce,
                    destination,
                    amount,
                    body: None,
                    state_init: None,
                }];

                let seqno_offset = nekoton::core::ton_wallet::wallet_v3::estimate_seqno_offset(
                    &SimpleClock,
                    &current_state,
                    &[],
                );

                nekoton::core::ton_wallet::wallet_v3::prepare_transfer(
                    &SimpleClock,
                    &public_key,
                    &current_state,
                    seqno_offset,
                    gifts,
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

                let gift = nekoton::core::ton_wallet::Gift {
                    flags: flags.into(),
                    bounce,
                    destination,
                    amount,
                    body: None,
                    state_init: None,
                };

                nekoton::core::ton_wallet::multisig::prepare_transfer(
                    &SimpleClock,
                    MultisigType::SafeMultisigWallet,
                    &public_key,
                    has_multiple_owners,
                    address.clone(),
                    gift,
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

        let data_to_sign = ton_abi::extend_signature_with_id(
            unsigned_message.hash(),
            self.ton_core.signature_id(),
        );
        let signature = key_pair.sign(&data_to_sign);
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

    pub async fn prepare_confirm_transaction(
        &self,
        transaction: TransactionConfirm,
        public_key: &[u8],
        private_key: &[u8],
    ) -> Result<(SentTransaction, SignedMessage), Error> {
        let public_key = PublicKey::from_bytes(public_key)?;
        let address = nekoton_utils::repack_address(&transaction.address.0)?;

        let account_workchain_id = address.workchain_id();
        let account_hex = address.address().to_hex_string();

        let unsigned_message = nekoton::core::ton_wallet::multisig::prepare_confirm_transaction(
            &SimpleClock,
            MultisigType::SafeMultisigWallet,
            &public_key,
            address,
            transaction.transaction_id,
            Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT),
        )?;

        let key_pair = Keypair {
            secret: SecretKey::from_bytes(private_key)?,
            public: public_key,
        };

        let data_to_sign = ton_abi::extend_signature_with_id(
            unsigned_message.hash(),
            self.ton_core.signature_id(),
        );
        let signature = key_pair.sign(&data_to_sign);
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

    pub async fn get_token_address_info(
        &self,
        owner: &MsgAddressInt,
        root_address: &MsgAddressInt,
    ) -> Result<NetworkTokenAddressData, Error> {
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
            utils::parse_last_transaction(&token_contract.last_transaction_id);

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

    pub async fn prepare_token_transaction(
        &self,
        input: &TokenTransactionSend,
        public_key: &[u8],
        private_key: &[u8],
        account_type: &AccountType,
        custodians: &Option<i32>,
    ) -> Result<(SentTransaction, SignedMessage), Error> {
        let owner = nekoton_utils::repack_address(&input.from_address.0)?;

        let token_owner_db = self
            .sqlx_client
            .get_token_address(
                owner.workchain_id(),
                owner.address().to_hex_string(),
                input.root_address.0.clone(),
            )
            .await?;
        let token_wallet = nekoton_utils::repack_address(&token_owner_db.address)?;

        let recipient = nekoton_utils::repack_address(&input.recipient_address.0)?;
        let destination = nekoton::core::models::TransferRecipient::OwnerWallet(recipient);

        let send_gas_to = match &input.send_gas_to {
            Some(send_gas_to) => nekoton_utils::repack_address(send_gas_to.0.as_str())?,
            None => owner.clone(),
        };

        let version = token_owner_db.version.into();

        let (value, _) = input.value.clone().as_bigint_and_exponent();
        let tokens = value.to_biguint().ok_or(TonClientError::ParseBigUint)?;

        let attached_amount = input.fee.to_u64().ok_or(TonClientError::ParseBigDecimal)?;

        // parse input payload
        let payload_cell = match &input.payload {
            None => None,
            Some(s) => {
                let bytes = base64::decode(s).map_err(anyhow::Error::from)?;
                let mut slice = &bytes[..];
                let tree_of_cells = deserialize_tree_of_cells(&mut slice)?;
                Some(tree_of_cells)
            }
        };

        let internal_message = prepare_token_transfer(
            owner.clone(),
            token_wallet,
            version,
            destination,
            tokens,
            send_gas_to,
            input.notify_receiver,
            attached_amount,
            payload_cell.unwrap_or_default(),
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
    ) -> Result<(SentTransaction, SignedMessage), Error> {
        let owner = nekoton_utils::repack_address(&input.from_address.0)?;

        let token_owner_db = self
            .sqlx_client
            .get_token_address(
                owner.workchain_id(),
                owner.address().to_hex_string(),
                input.root_address.0.clone(),
            )
            .await?;
        let token_wallet = nekoton_utils::repack_address(&token_owner_db.address)?;

        let send_gas_to = match &input.send_gas_to {
            Some(send_gas_to) => nekoton_utils::repack_address(send_gas_to.0.as_str())?,
            None => owner.clone(),
        };

        let callback_to = nekoton_utils::repack_address(input.callback_to.0.as_str())?;

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
    ) -> Result<(SentTransaction, SignedMessage), Error> {
        let owner = nekoton_utils::repack_address(&input.owner_address.0)?;
        let root_token = nekoton_utils::repack_address(&input.root_address.0)?;
        let recipient = nekoton_utils::repack_address(&input.recipient_address.0)?;

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
            Some(send_gas_to) => nekoton_utils::repack_address(send_gas_to.0.as_str())?,
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
        account: UInt256,
        signed_message: SignedMessage,
    ) -> Result<MessageStatus, Error> {
        let status = self
            .ton_core
            .send_ton_message(&account, &signed_message.message, signed_message.expire_at)
            .await?;

        Ok(status)
    }

    pub fn add_pending_message(
        &self,
        account: UInt256,
        message_hash: UInt256,
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

    pub async fn run_local(
        &self,
        contract_address: UInt256,
        function: ton_abi::Function,
        input: &[ton_abi::Token],
    ) -> anyhow::Result<Option<nekoton_abi::ExecutionOutput>> {
        use nekoton_abi::FunctionExt;

        let state = match self.ton_core.get_contract_state(&contract_address) {
            Ok(a) => a,
            Err(e) => {
                log::error!("Failed to get contract state: {e:?}");
                return Ok(None);
            }
        };

        function
            .run_local(&SimpleClock, state.account, input)
            .map(Some)
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
        function: Option<ton_abi::Function>,
        params: Option<Vec<ton_abi::Token>>,
    ) -> Result<SignedMessage, Error> {
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

        let public_key = PublicKey::from_bytes(public_key).unwrap_or_default();

        let key_pair = Keypair {
            secret: SecretKey::from_bytes(private_key)?,
            public: public_key,
        };

        let data_to_sign = ton_abi::extend_signature_with_id(
            unsigned_message.hash(),
            self.ton_core.signature_id(),
        );
        let signature = key_pair.sign(&data_to_sign);
        let signed_message = unsigned_message.sign(&signature.to_bytes())?;

        Ok(signed_message)
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
        function: Option<ton_abi::Function>,
        params: Option<Vec<ton_abi::Token>>,
    ) -> Result<Box<dyn UnsignedMessage>, Error> {
        let address = nekoton_utils::repack_address(sender_addr)?;
        let public_key = PublicKey::from_bytes(public_key)?;

        let expiration = Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT);

        let function_data = function.and_then(|x| {
            let tokens = params.unwrap_or_default();
            let (func, _) = MessageBuilder::new(&x).build();
            func.encode_internal_input(&tokens).ok()
        });

        let destination = nekoton_utils::repack_address(target_addr)?;
        let amount = value.to_u64().ok_or(TonClientError::ParseBigDecimal)?;
        let transfer_action = match account_type {
            AccountType::Wallet => {
                let account = UInt256::from_be_bytes(&address.address().get_bytestring(0));
                let current_state = self.ton_core.get_contract_state(&account)?.account;

                let gifts = vec![nekoton::core::ton_wallet::Gift {
                    flags: execution_flag,
                    bounce,
                    destination,
                    amount,
                    body: function_data.map(|x| x.into()),
                    state_init: None,
                }];

                let seqno_offset = nekoton::core::ton_wallet::wallet_v3::estimate_seqno_offset(
                    &SimpleClock,
                    &current_state,
                    &[],
                );

                nekoton::core::ton_wallet::wallet_v3::prepare_transfer(
                    &SimpleClock,
                    &public_key,
                    &current_state,
                    seqno_offset,
                    gifts,
                    expiration,
                )?
            }
            AccountType::SafeMultisig => {
                let has_multiple_owners = match custodians {
                    Some(custodians) => *custodians > 1,
                    None => return Err(TonClientError::CustodiansNotFound.into()),
                };

                let gift = nekoton::core::ton_wallet::Gift {
                    flags: execution_flag,
                    bounce,
                    destination,
                    amount,
                    body: function_data.map(|x| x.into()),
                    state_init: None,
                };

                nekoton::core::ton_wallet::multisig::prepare_transfer(
                    &SimpleClock,
                    MultisigType::SafeMultisigWallet,
                    &public_key,
                    has_multiple_owners,
                    address,
                    gift,
                    expiration,
                )?
            }
            AccountType::HighloadWallet => {
                return Err(TonServiceError::WrongInput("Invalid account type".to_string()).into())
            }
        };

        let unsigned_message = match transfer_action {
            TransferAction::Sign(unsigned_message) => unsigned_message,
            TransferAction::DeployFirst => {
                return Err(TonClientError::AccountNotDeployed(target_addr.to_string()).into())
            }
        };
        Ok(unsigned_message)
    }

    pub fn add_ton_account_subscription(&self, account: UInt256) {
        self.ton_core.add_ton_account_subscription([account])
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
    owner: MsgAddressInt,
    public_key: &[u8],
    private_key: &[u8],
    account_type: &AccountType,
    custodians: &Option<i32>,
    internal_message: InternalMessage,
) -> anyhow::Result<(SentTransaction, SignedMessage)> {
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

            let gift = nekoton::core::ton_wallet::Gift {
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

            let gifts = vec![nekoton::core::ton_wallet::Gift {
                flags: flags.into(),
                bounce,
                destination,
                amount,
                body,
                state_init: None,
            }];

            let seqno_offset = nekoton::core::ton_wallet::wallet_v3::estimate_seqno_offset(
                &SimpleClock,
                &current_state,
                &[],
            );

            nekoton::core::ton_wallet::wallet_v3::prepare_transfer(
                &SimpleClock,
                &public_key,
                &current_state,
                seqno_offset,
                gifts,
                expiration,
            )?
        }
        AccountType::SafeMultisig => {
            let has_multiple_owners = match custodians {
                Some(custodians) => *custodians > 1,
                None => return Err(TonClientError::CustodiansNotFound.into()),
            };

            let gift = nekoton::core::ton_wallet::Gift {
                flags: flags.into(),
                bounce,
                destination,
                amount,
                body,
                state_init: None,
            };

            nekoton::core::ton_wallet::multisig::prepare_transfer(
                &SimpleClock,
                MultisigType::SafeMultisigWallet,
                &public_key,
                has_multiple_owners,
                owner.clone(),
                gift,
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

    let data_to_sign =
        ton_abi::extend_signature_with_id(unsigned_message.hash(), ton_core.signature_id());
    let signature = key_pair.sign(&data_to_sign);
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
