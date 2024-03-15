use std::convert::TryInto;
use std::str::FromStr;
use std::sync::{Arc, Weak};

use bigdecimal::BigDecimal;
use http::StatusCode;
use nekoton::crypto::{SignedMessage, UnsignedMessage};
use nekoton_utils::{repack_address, unpack_std_smc_addr};
use serde_json::Value;
use ton_abi::contract::ABI_VERSION_2_2;
use ton_abi::{Param, Token, TokenValue};
use ton_block::{GetRepresentationHash, MsgAddressInt, Serializable};
use ton_types::{BuilderData, UInt256};
use uuid::Uuid;

use crate::api::*;
use crate::client::*;
use crate::models::*;
use crate::prelude::*;
use crate::sqlx_client::*;
use crate::utils::*;

#[derive(Clone)]
pub struct TonService {
    sqlx_client: SqlxClient,
    ton_api_client: Arc<TonClient>,
    callback_client: Arc<CallbackClient>,
    key: Arc<Vec<u8>>,
}

impl TonService {
    pub fn new(
        sqlx_client: SqlxClient,
        ton_api_client: Arc<TonClient>,
        callback_client: Arc<CallbackClient>,
        key: Vec<u8>,
    ) -> Self {
        let key = Arc::new(key);
        Self {
            sqlx_client,
            ton_api_client,
            callback_client,
            key,
        }
    }

    pub async fn start(self: &Arc<Self>) -> anyhow::Result<()> {
        // Get unprocessed sent transactions
        let transactions: Vec<TransactionDb> = self
            .sqlx_client
            .get_all_transactions_by_status(TonTransactionStatus::New)
            .await?;

        // Resend transactions
        for transaction in transactions {
            let account = UInt256::from_be_bytes(&hex::decode(transaction.account_hex.clone())?);
            let message_hash =
                UInt256::from_be_bytes(&hex::decode(transaction.message_hash.clone())?);
            let expire_at = transaction.created_at.timestamp() as u32 + DEFAULT_EXPIRATION_TIMEOUT;

            let rx = self
                .ton_api_client
                .add_pending_message(account, message_hash, expire_at)?;

            let ton_service = Arc::downgrade(self);
            self.spawn_background_task("Wait message", wait_message(ton_service, transaction, rx));
        }

        Ok(())
    }

    pub async fn create_address(
        &self,
        service_id: &ServiceId,
        input: CreateAddress,
    ) -> Result<AddressDb, Error> {
        let id = Uuid::new_v4();
        let key = self.key.as_slice().try_into()?;
        let address = self.ton_api_client.create_address(input).await?;

        let public_key = hex::encode(&address.public_key);
        let private_key = encrypt_private_key(&address.private_key, key, &id)?;

        let address = self
            .sqlx_client
            .create_address(CreateAddressInDb::new(
                address,
                id,
                *service_id,
                public_key,
                private_key,
            ))
            .await?;

        Ok(address)
    }

    pub async fn check_address(&self, address: Address) -> Result<bool, Error> {
        Ok(MsgAddressInt::from_str(&address.0).is_ok()
            || (unpack_std_smc_addr(&address.0, false).is_ok())
            || (unpack_std_smc_addr(&address.0, true).is_ok()))
    }

    pub async fn get_address_balance(
        &self,
        service_id: &ServiceId,
        address: Address,
    ) -> Result<(AddressDb, NetworkAddressData), Error> {
        let account = repack_address(&address.0)?;
        let address = self
            .sqlx_client
            .get_address(
                *service_id,
                account.workchain_id(),
                account.address().to_hex_string(),
            )
            .await?;
        let network = self.ton_api_client.get_address_info(&account).await?;

        Ok((address, network))
    }

    pub async fn get_address_info(
        &self,
        service_id: &ServiceId,
        address: Address,
    ) -> Result<AddressDb, Error> {
        let account = repack_address(&address.0)?;
        let address = self
            .sqlx_client
            .get_address(
                *service_id,
                account.workchain_id(),
                account.address().to_hex_string(),
            )
            .await?;

        Ok(address)
    }

    pub async fn create_send_transaction(
        self: &Arc<Self>,
        service_id: &ServiceId,
        input: TransactionSend,
    ) -> Result<TransactionDb, Error> {
        let address = repack_address(&input.from_address.0)?;
        let network = self.ton_api_client.get_address_info(&address).await?;

        for transaction_output in input.outputs.iter() {
            let (_, scale) = transaction_output.value.as_bigint_and_exponent();
            if scale != 0 {
                return Err(TonServiceError::WrongInput("Invalid value".to_string()).into());
            }
        }

        let balance = input
            .outputs
            .iter()
            .map(|o| o.value.clone())
            .sum::<BigDecimal>();
        if balance >= network.network_balance
            && input.outputs.iter().all(|o| {
                o.output_type.is_none() || o.output_type == Some(TransactionSendOutputType::Normal)
            })
        {
            return Err(TonServiceError::InsufficientBalance.into());
        }

        let address_db = self
            .sqlx_client
            .get_address(
                *service_id,
                address.workchain_id(),
                address.address().to_hex_string(),
            )
            .await?;

        let key = self.key.as_slice().try_into()?;

        let public_key = hex::decode(address_db.public_key.clone())?;
        let private_key = decrypt_private_key(&address_db.private_key, key, &address_db.id)?;

        if network.account_status == AccountStatus::UnInit {
            self.deploy_wallet(service_id, &address_db, &public_key, &private_key)
                .await?;
        }

        let (payload, signed_message) = self
            .ton_api_client
            .prepare_transaction(
                input,
                &public_key,
                &private_key,
                &address_db.account_type,
                &address_db.custodians,
            )
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_send_transaction(CreateSendTransaction::new(payload, *service_id))
            .await?;

        self.send_transaction(
            transaction.message_hash.clone(),
            transaction.account_hex.clone(),
            transaction.account_workchain_id,
            signed_message,
            true,
            true,
        )
        .await?;

        self.notify(service_id, event.into(), NotifyType::Transaction)
            .await?;

        Ok(transaction)
    }

    pub async fn create_confirm_transaction(
        self: &Arc<Self>,
        service_id: &ServiceId,
        input: TransactionConfirm,
    ) -> Result<TransactionDb, Error> {
        let address = repack_address(&input.address.0)?;

        let address_db = self
            .sqlx_client
            .get_address(
                *service_id,
                address.workchain_id(),
                address.address().to_hex_string(),
            )
            .await?;

        if address_db.account_type != AccountType::SafeMultisig {
            return Err(TonServiceError::WrongInput("Invalid account type".to_string()).into());
        }

        let key = self.key.as_slice().try_into()?;

        let public_key = hex::decode(address_db.public_key.clone())?;
        let private_key = decrypt_private_key(&address_db.private_key, key, &address_db.id)?;

        let network = self.ton_api_client.get_address_info(&address).await?;

        if network.account_status == AccountStatus::UnInit {
            self.deploy_wallet(service_id, &address_db, &public_key, &private_key)
                .await?;
        }

        let (payload, signed_message) = self
            .ton_api_client
            .prepare_confirm_transaction(input, &public_key, &private_key)
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_send_transaction(CreateSendTransaction::new(payload, *service_id))
            .await?;

        self.send_transaction(
            transaction.message_hash.clone(),
            transaction.account_hex.clone(),
            transaction.account_workchain_id,
            signed_message,
            true,
            true,
        )
        .await?;

        self.notify(service_id, event.into(), NotifyType::Transaction)
            .await?;

        Ok(transaction)
    }

    pub async fn create_receive_transaction(
        self: &Arc<Self>,
        input: CreateReceiveTransaction,
    ) -> Result<TransactionDb, Error> {
        let address = self
            .sqlx_client
            .get_address_by_workchain_hex(input.account_workchain_id, input.account_hex.clone())
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_receive_transaction(input, address.service_id)
            .await?;

        self.notify(&address.service_id, event.into(), NotifyType::Transaction)
            .await?;

        Ok(transaction)
    }

    pub async fn upsert_sent_transaction(
        self: &Arc<Self>,
        message_hash: String,
        account_workchain_id: i32,
        account_hex: String,
        input: UpdateSendTransaction,
    ) -> Result<TransactionDb, Error> {
        let address = self
            .sqlx_client
            .get_address_by_workchain_hex(account_workchain_id, account_hex.clone())
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .upsert_send_transaction(
                address.service_id,
                message_hash,
                account_workchain_id,
                account_hex,
                input,
            )
            .await?;

        self.notify(&address.service_id, event.into(), NotifyType::Transaction)
            .await?;

        Ok(transaction)
    }

    pub async fn update_token_transaction(
        self: &Arc<Self>,
        owner_message_hash: String,
        account_workchain_id: i32,
        account_hex: String,
        messages_hash: Option<Value>,
    ) -> Result<(), Error> {
        if let Some(messages_hash) = messages_hash {
            let messages_hash: Vec<String> = serde_json::from_value(messages_hash.clone())?;

            let address = self
                .sqlx_client
                .get_address_by_workchain_hex(account_workchain_id, account_hex.clone())
                .await?;

            for in_message_hash in messages_hash {
                if let Some(event) = self
                    .sqlx_client
                    .update_token_transaction(
                        address.service_id,
                        &in_message_hash,
                        Some(owner_message_hash.clone()),
                    )
                    .await?
                {
                    let _ = self
                        .notify(
                            &address.service_id,
                            event.into(),
                            NotifyType::TokenTransaction,
                        )
                        .await;
                }
            }
        }

        Ok(())
    }

    pub async fn get_transaction_by_mh(
        &self,
        service_id: &ServiceId,
        message_hash: &str,
    ) -> Result<TransactionDb, Error> {
        let transaction = self
            .sqlx_client
            .get_transaction_by_mh(*service_id, message_hash)
            .await?;

        Ok(transaction)
    }

    pub async fn get_transaction_by_h(
        &self,
        service_id: &ServiceId,
        transaction_hash: &str,
    ) -> Result<TransactionDb, Error> {
        let transaction = self
            .sqlx_client
            .get_transaction_by_h(*service_id, transaction_hash)
            .await?;

        Ok(transaction)
    }

    pub async fn get_transaction_by_id(
        &self,
        service_id: &ServiceId,
        id: &Uuid,
    ) -> Result<TransactionDb, Error> {
        let transaction = self
            .sqlx_client
            .get_transaction_by_id(*service_id, id)
            .await?;

        Ok(transaction)
    }

    pub async fn get_event_by_id(
        &self,
        service_id: &ServiceId,
        id: &Uuid,
    ) -> Result<TransactionEventDb, Error> {
        let event = self.sqlx_client.get_event_by_id(*service_id, id).await?;
        Ok(event)
    }

    pub async fn search_transaction(
        &self,
        service_id: &ServiceId,
        payload: &TransactionsSearch,
    ) -> Result<Vec<TransactionDb>, Error> {
        let transaction = self
            .sqlx_client
            .get_all_transactions(*service_id, payload)
            .await?;

        Ok(transaction)
    }

    pub async fn search_events(
        &self,
        service_id: &ServiceId,
        payload: &TransactionsEventsSearch,
    ) -> Result<Vec<TransactionEventDb>, Error> {
        let events = self
            .sqlx_client
            .get_all_transaction_events(*service_id, payload)
            .await?;

        Ok(events)
    }

    pub async fn mark_event(
        &self,
        service_id: &ServiceId,
        id: &Uuid,
    ) -> Result<TransactionEventDb, Error> {
        let event = self
            .sqlx_client
            .update_event_status_of_transaction_event_by_id(
                *service_id,
                *id,
                TonEventStatus::Notified,
            )
            .await?;

        Ok(event)
    }

    pub async fn mark_all_events(
        &self,
        service_id: &ServiceId,
        event_status: Option<TonEventStatus>,
    ) -> Result<Vec<TransactionEventDb>, Error> {
        let events = self
            .sqlx_client
            .update_event_status_of_transactions_event_by_status(
                *service_id,
                event_status,
                TonEventStatus::Notified,
            )
            .await?;

        Ok(events)
    }

    pub async fn get_tokens_transaction_by_mh(
        &self,
        service_id: &ServiceId,
        message_hash: &str,
    ) -> Result<TokenTransactionFromDb, Error> {
        let transaction = self
            .sqlx_client
            .get_token_transaction_by_mh(*service_id, message_hash)
            .await?;

        Ok(transaction)
    }

    pub async fn get_tokens_transaction_by_id(
        &self,
        service_id: &ServiceId,
        id: &Uuid,
    ) -> Result<TokenTransactionFromDb, Error> {
        let transaction = self
            .sqlx_client
            .get_token_transaction_by_id(*service_id, id)
            .await?;

        Ok(transaction)
    }

    pub async fn search_token_events(
        &self,
        service_id: &ServiceId,
        payload: &TokenTransactionsEventsSearch,
    ) -> Result<Vec<TokenTransactionEventDb>, Error> {
        let events = self
            .sqlx_client
            .get_all_token_transaction_events(*service_id, payload)
            .await?;

        Ok(events)
    }

    pub async fn mark_token_event(
        &self,
        service_id: &ServiceId,
        id: &Uuid,
    ) -> Result<TokenTransactionEventDb, Error> {
        let event = self
            .sqlx_client
            .update_event_status_of_token_transaction_event_by_id(
                *service_id,
                *id,
                TonEventStatus::Notified,
            )
            .await?;

        Ok(event)
    }

    pub async fn get_token_address_balance(
        &self,
        service_id: &ServiceId,
        address: &Address,
    ) -> Result<Vec<(TokenBalanceFromDb, NetworkTokenAddressData)>, Error> {
        let account = repack_address(&address.0)?;
        let balances = self
            .sqlx_client
            .get_token_balances(
                *service_id,
                account.workchain_id(),
                account.address().to_hex_string(),
            )
            .await?;

        let mut result = Vec::with_capacity(balances.len());
        for balance in balances {
            let root_address = repack_address(&balance.root_address)?;

            let network = self
                .ton_api_client
                .get_token_address_info(&account, &root_address)
                .await?;

            result.push((balance, network));
        }

        Ok(result)
    }

    pub async fn create_send_token_transaction(
        self: &Arc<Self>,
        service_id: &ServiceId,
        input: &TokenTransactionSend,
    ) -> Result<TransactionDb, Error> {
        let (_, scale) = input.value.as_bigint_and_exponent();
        if scale != 0 {
            return Err(TonServiceError::WrongInput("Invalid value".to_string()).into());
        }

        let owner = repack_address(&input.from_address.0)?;
        let address_db = self
            .sqlx_client
            .get_address(
                *service_id,
                owner.workchain_id(),
                owner.address().to_hex_string(),
            )
            .await?;

        if address_db.balance < input.fee {
            log::error!(
                "Address balance is not enough to pay fee for token transfer. Balance: {}. Fee: {}",
                address_db.balance,
                input.fee
            );
            return Err(TonServiceError::InsufficientBalance.into());
        }

        let token_balance = self
            .sqlx_client
            .get_token_balance(
                *service_id,
                owner.workchain_id(),
                owner.address().to_hex_string(),
                input.root_address.0.clone(),
            )
            .await?;

        if token_balance.balance < input.value {
            log::error!(
                "Token balance is not enough to make request; Balance: {}. Sent amount: {}",
                token_balance.balance,
                input.value
            );
            return Err(TonServiceError::InsufficientBalance.into());
        }

        let key = self.key.as_slice().try_into()?;

        let public_key = hex::decode(address_db.public_key.clone())?;
        let private_key = decrypt_private_key(&address_db.private_key, key, &address_db.id)?;

        let owner_network = self.ton_api_client.get_address_info(&owner).await?;

        if owner_network.account_status == AccountStatus::UnInit {
            self.deploy_wallet(service_id, &address_db, &public_key, &private_key)
                .await?;
        }

        let (payload, signed_message) = self
            .ton_api_client
            .prepare_token_transaction(
                input,
                &public_key,
                &private_key,
                &address_db.account_type,
                &address_db.custodians,
            )
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_send_transaction(CreateSendTransaction::new(payload, *service_id))
            .await?;

        self.send_transaction(
            transaction.message_hash.clone(),
            transaction.account_hex.clone(),
            transaction.account_workchain_id,
            signed_message,
            true,
            true,
        )
        .await?;

        self.notify(service_id, event.into(), NotifyType::Transaction)
            .await?;

        Ok(transaction)
    }

    pub async fn create_burn_token_transaction(
        self: &Arc<Self>,
        service_id: &ServiceId,
        input: &TokenTransactionBurn,
    ) -> Result<TransactionDb, Error> {
        let (_, scale) = input.value.as_bigint_and_exponent();
        if scale != 0 {
            return Err(TonServiceError::WrongInput("Invalid value".to_string()).into());
        }

        let owner = repack_address(&input.from_address.0)?;
        let address_db = self
            .sqlx_client
            .get_address(
                *service_id,
                owner.workchain_id(),
                owner.address().to_hex_string(),
            )
            .await?;

        if address_db.balance < input.fee {
            log::error!(
                "Address balance is not enough to pay fee for token transfer. Balance: {}. Fee: {}",
                address_db.balance,
                input.fee
            );
            return Err(TonServiceError::InsufficientBalance.into());
        }

        let token_balance = self
            .sqlx_client
            .get_token_balance(
                *service_id,
                owner.workchain_id(),
                owner.address().to_hex_string(),
                input.root_address.0.clone(),
            )
            .await?;

        if token_balance.balance < input.value {
            log::error!(
                "Token balance is not enough to make request; Balance: {}. Sent amount: {}",
                token_balance.balance,
                input.value
            );
            return Err(TonServiceError::InsufficientBalance.into());
        }

        let key = self.key.as_slice().try_into()?;

        let public_key = hex::decode(address_db.public_key.clone())?;
        let private_key = decrypt_private_key(&address_db.private_key, key, &address_db.id)?;

        let owner_network = self.ton_api_client.get_address_info(&owner).await?;

        if owner_network.account_status == AccountStatus::UnInit {
            self.deploy_wallet(service_id, &address_db, &public_key, &private_key)
                .await?;
        }

        let (payload, signed_message) = self
            .ton_api_client
            .prepare_token_burn(
                input,
                &public_key,
                &private_key,
                &address_db.account_type,
                &address_db.custodians,
            )
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_send_transaction(CreateSendTransaction::new(payload, *service_id))
            .await?;

        self.send_transaction(
            transaction.message_hash.clone(),
            transaction.account_hex.clone(),
            transaction.account_workchain_id,
            signed_message,
            true,
            true,
        )
        .await?;

        self.notify(service_id, event.into(), NotifyType::Transaction)
            .await?;

        Ok(transaction)
    }

    pub async fn create_mint_token_transaction(
        self: &Arc<Self>,
        service_id: &ServiceId,
        input: &TokenTransactionMint,
    ) -> Result<TransactionDb, Error> {
        let (_, scale) = input.value.as_bigint_and_exponent();
        if scale != 0 {
            return Err(TonServiceError::WrongInput("Invalid value".to_string()).into());
        }

        let (_, scale) = input.deploy_wallet_value.as_bigint_and_exponent();
        if scale != 0 {
            return Err(TonServiceError::WrongInput("Invalid value".to_string()).into());
        }

        let owner = repack_address(&input.owner_address.0)?;
        let address_db = self
            .sqlx_client
            .get_address(
                *service_id,
                owner.workchain_id(),
                owner.address().to_hex_string(),
            )
            .await?;

        if address_db.balance < input.fee {
            log::error!(
                "Address balance is not enough to pay fee for token transfer. Balance: {}. Fee: {}",
                address_db.balance,
                input.fee
            );
            return Err(TonServiceError::InsufficientBalance.into());
        }

        let key = self.key.as_slice().try_into()?;

        let public_key = hex::decode(address_db.public_key.clone())?;
        let private_key = decrypt_private_key(&address_db.private_key, key, &address_db.id)?;

        let (payload, signed_message) = self
            .ton_api_client
            .prepare_token_mint(
                input,
                &public_key,
                &private_key,
                &address_db.account_type,
                &address_db.custodians,
            )
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_send_transaction(CreateSendTransaction::new(payload, *service_id))
            .await?;

        self.send_transaction(
            transaction.message_hash.clone(),
            transaction.account_hex.clone(),
            transaction.account_workchain_id,
            signed_message,
            true,
            true,
        )
        .await?;

        self.notify(service_id, event.into(), NotifyType::Transaction)
            .await?;

        Ok(transaction)
    }

    pub async fn create_receive_token_transaction(
        self: &Arc<Self>,
        input: CreateTokenTransaction,
    ) -> Result<TokenTransactionFromDb, Error> {
        let address = self
            .sqlx_client
            .get_address_by_workchain_hex(input.account_workchain_id, input.account_hex.clone())
            .await?;

        let (transaction, event) = self
            .sqlx_client
            .create_token_transaction(input, address.service_id)
            .await?;

        if transaction.direction == TonTransactionDirection::Receive
            || transaction.owner_message_hash.is_some()
        {
            self.notify(
                &address.service_id,
                event.into(),
                NotifyType::TokenTransaction,
            )
            .await?;
        }

        Ok(transaction)
    }

    pub async fn get_metrics(&self) -> Result<Metrics, Error> {
        let metrics = self.ton_api_client.get_metrics().await?;
        Ok(metrics)
    }

    pub async fn execute_contract_function(
        self: &Arc<Self>,
        account_addr: &str,
        function_name: &str,
        inputs: Vec<InputParam>,
        outputs: Vec<Param>,
        headers: Vec<Param>,
        responsible: bool,
    ) -> Result<Value, Error> {
        let account_addr = UInt256::from_str(account_addr)?;

        let input_params: Vec<Param> = inputs.iter().map(|x| x.param.clone()).collect();

        let function = nekoton_abi::FunctionBuilder::new(function_name)
            .abi_version(ABI_VERSION_2_2)
            .headers(headers)
            .inputs(input_params)
            .outputs(outputs)
            .build();

        let input = parse_abi_tokens(inputs)?;
        let output = match self
            .ton_api_client
            .run_local(account_addr, function, input.as_slice(), responsible)
            .await?
        {
            Some(output) => output,
            None => return Err(TonServiceError::ExecuteContract.into()),
        };

        let tokens = match output.tokens {
            Some(tokens) => {
                if tokens.is_empty() {
                    log::warn!("No response tokens in execution output")
                }
                tokens
            }
            None => return Err(TonServiceError::ExecuteContract.into()),
        };

        let res = nekoton_abi::make_abi_tokens(tokens.as_slice())?;
        Ok(res)
    }

    pub async fn prepare_and_send_signed_generic_message(
        self: &Arc<Self>,
        service_id: &ServiceId,
        sender_addr: &str,
        target_addr: &str,
        execution_flag: u8,
        value: BigDecimal,
        bounce: bool,
        account_type: &AccountType,
        custodians: &Option<i32>,
        function_details: Option<FunctionDetails>,
        transaction_id: Uuid,
    ) -> Result<TransactionDb, Error> {
        let (function, values) = match function_details {
            Some(details) => {
                let function = nekoton_abi::FunctionBuilder::new(&details.function_name)
                    .abi_version(ABI_VERSION_2_2)
                    .headers(details.headers)
                    .inputs(
                        details
                            .input_params
                            .clone()
                            .into_iter()
                            .map(|x| x.param)
                            .collect::<Vec<Param>>(),
                    )
                    .outputs(details.output_params)
                    .build();

                let tokens = parse_abi_tokens(details.input_params)?;

                (Some(function), Some(tokens))
            }
            None => (None, None),
        };

        let sender = repack_address(sender_addr)?;

        let address_db = self
            .sqlx_client
            .get_address(
                *service_id,
                sender.workchain_id(),
                sender.address().to_hex_string(),
            )
            .await?;

        let key = self.key.as_slice().try_into()?;

        let public_key = hex::decode(address_db.public_key.clone())?;
        let private_key = decrypt_private_key(&address_db.private_key, key, &address_db.id)?;

        let signed_message = self
            .ton_api_client
            .prepare_signed_generic_message(
                sender_addr,
                &public_key,
                &private_key,
                target_addr,
                execution_flag,
                value.clone(),
                bounce,
                account_type,
                custodians,
                function,
                values,
            )
            .await?;

        let sent_transaction = SentTransaction {
            id: transaction_id,
            message_hash: signed_message.message.hash()?.to_hex_string(),
            account_workchain_id: sender.workchain_id(),
            account_hex: sender.address().to_hex_string(),
            original_value: Some(value),
            original_outputs: None,
            aborted: false,
            bounce,
        };

        let (transaction, event) = self
            .sqlx_client
            .create_send_transaction(CreateSendTransaction::new(sent_transaction, *service_id))
            .await?;

        self.send_transaction(
            transaction.message_hash.clone(),
            transaction.account_hex.clone(),
            transaction.account_workchain_id,
            signed_message,
            true,
            true,
        )
        .await?;

        self.notify(service_id, event.into(), NotifyType::Transaction)
            .await?;

        Ok(transaction)
    }

    pub async fn prepare_generic_message(
        self: &Arc<Self>,
        sender_addr: &str,
        public_key: &[u8],
        target_addr: &str,
        execution_flag: u8,
        value: BigDecimal,
        bounce: bool,
        account_type: &AccountType,
        custodians: &Option<i32>,
        function_details: Option<FunctionDetails>,
    ) -> Result<Box<dyn UnsignedMessage>, Error> {
        let (function, values) = match function_details {
            Some(details) => {
                let function = nekoton_abi::FunctionBuilder::new(&details.function_name)
                    .abi_version(ABI_VERSION_2_2)
                    .headers(details.headers)
                    .inputs(
                        details
                            .input_params
                            .clone()
                            .into_iter()
                            .map(|x| x.param)
                            .collect::<Vec<Param>>(),
                    )
                    .outputs(details.output_params)
                    .build();

                let tokens = parse_abi_tokens(details.input_params)?;

                (Some(function), Some(tokens))
            }
            None => (None, None),
        };

        let result = self
            .ton_api_client
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
                values,
            )
            .await?;

        Ok(result)
    }

    pub fn encode_tvm_cell(&self, data: Vec<InputParam>) -> Result<String, Error> {
        let mut tokens: Vec<Token> = Vec::new();
        for d in data {
            let token_value =
                ton_abi::token::Tokenizer::tokenize_parameter(&d.param.kind, &d.value)?;
            let token = Token::new(&d.param.name, token_value);
            tokens.push(token);
        }
        let initial = if tokens.is_empty() {
            BuilderData::default()
        } else {
            TokenValue::pack_values_into_chain(
                tokens.as_slice(),
                Default::default(),
                &ABI_VERSION_2_2,
            )?
        };
        let cell = initial.into_cell()?;
        Ok(base64::encode(cell.write_to_bytes()?))
    }

    pub async fn send_signed_message(
        self: &Arc<Self>,
        sender_addr: String,
        hash: String,
        msg: SignedMessage,
    ) -> Result<String, Error> {
        let addr = MsgAddressInt::from_str(&sender_addr)?;
        self.ton_api_client
            .add_ton_account_subscription(addr.hash()?);

        self.send_transaction(
            hash,
            addr.address().to_hex_string(),
            addr.workchain_id(),
            msg.clone(),
            true,
            false,
        )
        .await?;

        let hash = msg.message.hash().map(|x| x.to_hex_string())?;

        Ok(hash)
    }

    pub async fn set_callback(
        &self,
        service_id: &ServiceId,
        callback: String,
    ) -> Result<String, Error> {
        let id = Uuid::new_v4();

        self
            .sqlx_client
            .set_callback(ApiServiceCallbackDb::new(
                id,
                *service_id,
                callback.clone(),
            ))
            .await?;

        Ok(callback)
    }

    async fn notify(
        self: &Arc<Self>,
        service_id: &ServiceId,
        payload: AccountTransactionEvent,
        notify_type: NotifyType,
    ) -> Result<(), Error> {
        let ton_service = Arc::downgrade(self);
        self.spawn_background_task(
            "Send notification",
            send_notification(ton_service, *service_id, notify_type, payload),
        );

        Ok(())
    }

    async fn send_transaction(
        self: &Arc<Self>,
        message_hash: String,
        account_hex: String,
        account_workchain_id: i32,
        signed_message: SignedMessage,
        non_blocking: bool,
        with_db_update: bool,
    ) -> Result<(), Error> {
        let ton_service = Arc::downgrade(self);

        match non_blocking {
            false => {
                send_transaction(
                    ton_service,
                    message_hash,
                    account_hex,
                    account_workchain_id,
                    signed_message,
                    with_db_update,
                )
                .await?
            }
            true => {
                self.spawn_background_task(
                    "Send transaction",
                    send_transaction(
                        ton_service,
                        message_hash,
                        account_hex,
                        account_workchain_id,
                        signed_message,
                        with_db_update,
                    ),
                );
            }
        }

        Ok(())
    }

    async fn deploy_wallet(
        self: &Arc<Self>,
        service_id: &ServiceId,
        address: &AddressDb,
        public_key: &[u8],
        private_key: &[u8],
    ) -> Result<(), Error> {
        let payload = self
            .ton_api_client
            .prepare_deploy(address, public_key, private_key)
            .await?;

        if let Some((payload, signed_message)) = payload {
            let (transaction, event) = self
                .sqlx_client
                .create_send_transaction(CreateSendTransaction::new(payload, *service_id))
                .await?;

            self.send_transaction(
                transaction.message_hash,
                transaction.account_hex,
                transaction.account_workchain_id,
                signed_message,
                false,
                true,
            )
            .await?;

            self.notify(service_id, event.into(), NotifyType::Transaction)
                .await?;
        }

        Ok(())
    }

    /// Waits future in background. In case of error does nothing but logging
    fn spawn_background_task<F>(self: &Arc<Self>, name: &'static str, fut: F)
    where
        F: Future<Output = Result<(), Error>> + Send + 'static,
    {
        tokio::spawn(async move {
            if let Err(e) = fut.await {
                log::error!("Failed to {}: {:?}", name, e);
            }
        });
    }
}

async fn wait_message(
    ton_service: Weak<TonService>,
    transaction: TransactionDb,
    rx: tokio::sync::oneshot::Receiver<MessageStatus>,
) -> Result<(), Error> {
    match rx.await? {
        MessageStatus::Delivered => {
            log::info!("Successfully sent message `{}`", transaction.message_hash)
        }
        MessageStatus::Expired => {
            let ton_service = match ton_service.upgrade() {
                Some(ton_service) => ton_service,
                None => return Err(TonServiceError::ServiceUnavailable.into()),
            };

            ton_service
                .upsert_sent_transaction(
                    transaction.message_hash,
                    transaction.account_workchain_id,
                    transaction.account_hex,
                    UpdateSendTransaction::error("Expired".to_string()),
                )
                .await?;
        }
    }

    Ok(())
}

async fn send_notification(
    ton_service: Weak<TonService>,
    service_id: ServiceId,
    notify_type: NotifyType,
    payload: AccountTransactionEvent,
) -> Result<(), Error> {
    let ton_service = match ton_service.upgrade() {
        Some(ton_service) => ton_service,
        None => return Err(TonServiceError::ServiceUnavailable.into()),
    };

    let sqlx_client = &ton_service.sqlx_client;
    let callback_client = &ton_service.callback_client;

    let url = sqlx_client.get_callback(service_id).await?;
    let secret = sqlx_client
        .get_key_by_service_id(&service_id)
        .await
        .map(|k| k.secret)?;

    let event_status = match callback_client.send(url, payload.clone(), secret).await {
        Err(_) => TonEventStatus::Error,
        Ok(_) => TonEventStatus::Notified,
    };

    match notify_type {
        NotifyType::Transaction => {
            sqlx_client
                .update_event_status_of_transaction_event(
                    payload.message_hash,
                    payload.account.workchain_id,
                    payload.account.hex.into(),
                    event_status,
                )
                .await?;
        }
        NotifyType::TokenTransaction => {
            sqlx_client
                .update_event_status_of_token_transaction_event(
                    payload.message_hash,
                    payload.account.workchain_id,
                    payload.account.hex.into(),
                    event_status,
                )
                .await?;
        }
    }

    Ok(())
}

async fn send_transaction(
    ton_service: Weak<TonService>,
    message_hash: String,
    account_hex: String,
    account_workchain_id: i32,
    signed_message: SignedMessage,
    with_db_update: bool,
) -> Result<(), Error> {
    let ton_service = match ton_service.upgrade() {
        Some(ton_service) => ton_service,
        None => return Err(TonServiceError::ServiceUnavailable.into()),
    };

    let account = UInt256::from_be_bytes(&hex::decode(&account_hex)?);

    let status = ton_service
        .ton_api_client
        .send_transaction(account, signed_message)
        .await?;

    if status == MessageStatus::Expired && with_db_update {
        ton_service
            .upsert_sent_transaction(
                message_hash,
                account_workchain_id,
                account_hex,
                UpdateSendTransaction::error("Expired".to_string()),
            )
            .await?;
    }

    Ok(())
}

fn parse_abi_tokens(params: Vec<InputParam>) -> Result<Vec<Token>, Error> {
    let mut tokens = Vec::<Token>::new();
    for i in params {
        let token = nekoton_abi::parse_abi_token(&i.param, i.value)?;
        tokens.push(token);
    }

    Ok(tokens)
}

enum NotifyType {
    Transaction,
    TokenTransaction,
}

#[derive(thiserror::Error, Debug)]
pub enum TonServiceError {
    #[error("Invalid request: `{0}`")]
    WrongInput(String),
    #[error("Service unavailable")]
    ServiceUnavailable,
    #[error("Insufficient balance")]
    InsufficientBalance,
    #[error("Execute contract")]
    ExecuteContract,
}

impl TonServiceError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            TonServiceError::WrongInput(_) | TonServiceError::InsufficientBalance => {
                StatusCode::BAD_REQUEST
            }
            TonServiceError::ServiceUnavailable | TonServiceError::ExecuteContract => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}
