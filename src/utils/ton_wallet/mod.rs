use std::borrow::Cow;
use std::convert::TryFrom;
use std::num::NonZeroU8;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use ed25519_dalek::PublicKey;
use serde::{Deserialize, Serialize};
use ton_block::MsgAddressInt;
use ton_types::{BuilderData, SliceData, UInt256};

use nekoton_abi::*;
use nekoton_utils::*;
use tycho_types::cell::Cell;
use tycho_types::models::{IntAddr, StateInit, StdAddr};

pub use self::multisig::MultisigType;
use super::models::{
    ContractState, Expiration, MessageFlags, MultisigPendingTransaction, MultisigPendingUpdate,
    PendingTransaction, Transaction, TransactionAdditionalInfo, TransactionWithData,
    TransactionsBatchInfo,
};
use super::{ContractSubscription, PollingMethod};
use crate::core::parsing::*;
use crate::core::InternalMessage;
use crate::crypto::UnsignedMessage;
use crate::models::ExpireAt;
use crate::transport::models::{ExistingContract, RawContractState, RawTransaction};
use crate::transport::Transport;

pub mod ever_wallet;
pub mod highload_wallet_v2;
pub mod multisig;
pub mod wallet_v3;
pub mod wallet_v3v4;
pub mod wallet_v5r1;

pub const DEFAULT_WORKCHAIN: i8 = 0;

pub struct TonWallet {
    clock: Arc<dyn Clock>,
    public_key: PublicKey,
    wallet_type: WalletType,
    contract_subscription: ContractSubscription,
    handler: Arc<dyn TonWalletSubscriptionHandler>,
    wallet_data: WalletData,
}

impl TonWallet {
    pub async fn subscribe(
        clock: Arc<dyn Clock>,
        transport: Arc<dyn Transport>,
        workchain: i8,
        public_key: PublicKey,
        wallet_type: WalletType,
        handler: Arc<dyn TonWalletSubscriptionHandler>,
    ) -> Result<Self> {
        let address = compute_address(&public_key, wallet_type, workchain);

        let mut wallet_data = WalletData::default();

        let contract_subscription = ContractSubscription::subscribe(
            clock.clone(),
            transport,
            address,
            &mut make_contract_state_handler(
                clock.as_ref(),
                handler.as_ref(),
                &public_key,
                wallet_type,
                &mut wallet_data,
            ),
            Some(&mut make_transactions_handler(
                handler.as_ref(),
                wallet_type,
            )),
        )
        .await?;

        Ok(Self {
            clock,
            public_key,
            wallet_type,
            contract_subscription,
            handler,
            wallet_data,
        })
    }

    pub async fn subscribe_by_address(
        clock: Arc<dyn Clock>,
        transport: Arc<dyn Transport>,
        address: MsgAddressInt,
        handler: Arc<dyn TonWalletSubscriptionHandler>,
    ) -> Result<Self> {
        let (public_key, wallet_type) = match transport.get_contract_state(&address).await? {
            RawContractState::Exists(contract) => extract_wallet_init_data(&contract)?,
            RawContractState::NotExists { .. } => {
                return Err(TonWalletError::AccountNotExists.into())
            }
        };

        let mut wallet_data = WalletData::default();

        let contract_subscription = ContractSubscription::subscribe(
            clock.clone(),
            transport,
            address,
            &mut make_contract_state_handler(
                clock.as_ref(),
                handler.as_ref(),
                &public_key,
                wallet_type,
                &mut wallet_data,
            ),
            Some(&mut make_transactions_handler(
                handler.as_ref(),
                wallet_type,
            )),
        )
        .await?;

        Ok(Self {
            clock,
            public_key,
            wallet_type,
            contract_subscription,
            handler,
            wallet_data,
        })
    }

    pub async fn subscribe_by_existing(
        clock: Arc<dyn Clock>,
        transport: Arc<dyn Transport>,
        existing_wallet: ExistingWalletInfo,
        handler: Arc<dyn TonWalletSubscriptionHandler>,
    ) -> Result<Self> {
        let mut wallet_data = WalletData::default();

        let contract_subscription = ContractSubscription::subscribe(
            clock.clone(),
            transport,
            existing_wallet.address,
            &mut make_contract_state_handler(
                clock.as_ref(),
                handler.as_ref(),
                &existing_wallet.public_key,
                existing_wallet.wallet_type,
                &mut wallet_data,
            ),
            Some(&mut make_transactions_handler(
                handler.as_ref(),
                existing_wallet.wallet_type,
            )),
        )
        .await?;

        Ok(Self {
            clock,
            public_key: existing_wallet.public_key,
            wallet_type: existing_wallet.wallet_type,
            contract_subscription,
            handler,
            wallet_data,
        })
    }

    pub fn make_state_init(&self) -> Result<ton_block::StateInit> {
        match self.wallet_type {
            WalletType::Multisig(multisig_type) => Ok(multisig::prepare_state_init(
                &self.public_key,
                multisig_type,
            )),
            WalletType::WalletV3 => wallet_v3::prepare_state_init(&self.public_key),
            WalletType::WalletV3R1 => {
                wallet_v3v4::prepare_state_init(&self.public_key, wallet_v3v4::WalletVersion::V3R1)
            }
            WalletType::WalletV3R2 => {
                wallet_v3v4::prepare_state_init(&self.public_key, wallet_v3v4::WalletVersion::V3R2)
            }
            WalletType::WalletV4R1 => {
                wallet_v3v4::prepare_state_init(&self.public_key, wallet_v3v4::WalletVersion::V4R1)
            }
            WalletType::WalletV4R2 => {
                wallet_v3v4::prepare_state_init(&self.public_key, wallet_v3v4::WalletVersion::V4R2)
            }
            WalletType::WalletV5R1 => wallet_v5r1::prepare_state_init(&self.public_key),
            WalletType::EverWallet => ever_wallet::prepare_state_init(&self.public_key),
            WalletType::HighloadWalletV2 => {
                highload_wallet_v2::prepare_state_init(&self.public_key)
            }
        }
    }

    pub fn contract_subscription(&self) -> &ContractSubscription {
        &self.contract_subscription
    }

    pub fn workchain(&self) -> i8 {
        self.contract_subscription.address().workchain_id() as i8
    }

    pub fn address(&self) -> &MsgAddressInt {
        self.contract_subscription.address()
    }

    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    pub fn wallet_type(&self) -> WalletType {
        self.wallet_type
    }

    pub fn contract_state(&self) -> &ContractState {
        self.contract_subscription.contract_state()
    }

    pub fn pending_transactions(&self) -> &[PendingTransaction] {
        self.contract_subscription.pending_transactions()
    }

    pub fn polling_method(&self) -> PollingMethod {
        self.contract_subscription.polling_method()
    }

    pub fn details(&self) -> TonWalletDetails {
        self.wallet_data
            .details
            .unwrap_or_else(|| self.wallet_type.details())
    }

    pub fn get_unconfirmed_transactions(&self) -> &[MultisigPendingTransaction] {
        &self.wallet_data.unconfirmed_transactions
    }

    pub fn get_unconfirmed_updates(&self) -> &[MultisigPendingUpdate] {
        &self.wallet_data.unconfirmed_updates
    }

    pub fn get_custodians(&self) -> &Option<Vec<UInt256>> {
        &self.wallet_data.custodians
    }

    pub fn prepare_deploy(&self, expiration: Expiration) -> Result<Box<dyn UnsignedMessage>> {
        match self.wallet_type {
            WalletType::Multisig(multisig_type) => multisig::prepare_deploy(
                self.clock.as_ref(),
                &self.public_key,
                multisig_type,
                self.workchain(),
                expiration,
                multisig::DeployParams::single_custodian(&self.public_key),
            ),
            WalletType::WalletV3 => wallet_v3::prepare_deploy(
                self.clock.as_ref(),
                &self.public_key,
                self.workchain(),
                expiration,
            ),
            WalletType::WalletV3R1 => wallet_v3v4::prepare_deploy(
                self.clock.as_ref(),
                &self.public_key,
                self.workchain(),
                expiration,
                wallet_v3v4::WalletVersion::V3R1,
            ),
            WalletType::WalletV3R2 => wallet_v3v4::prepare_deploy(
                self.clock.as_ref(),
                &self.public_key,
                self.workchain(),
                expiration,
                wallet_v3v4::WalletVersion::V3R2,
            ),
            WalletType::WalletV4R1 => wallet_v3v4::prepare_deploy(
                self.clock.as_ref(),
                &self.public_key,
                self.workchain(),
                expiration,
                wallet_v3v4::WalletVersion::V4R1,
            ),
            WalletType::WalletV4R2 => wallet_v3v4::prepare_deploy(
                self.clock.as_ref(),
                &self.public_key,
                self.workchain(),
                expiration,
                wallet_v3v4::WalletVersion::V4R2,
            ),
            WalletType::WalletV5R1 => wallet_v5r1::prepare_deploy(
                self.clock.as_ref(),
                &self.public_key,
                self.workchain(),
                expiration,
            ),
            WalletType::EverWallet => ever_wallet::prepare_deploy(
                self.clock.as_ref(),
                &self.public_key,
                self.workchain(),
                expiration,
            ),
            WalletType::HighloadWalletV2 => highload_wallet_v2::prepare_deploy(
                self.clock.as_ref(),
                &self.public_key,
                self.workchain(),
                expiration,
            ),
        }
    }

    pub fn prepare_deploy_with_multiple_owners(
        &self,
        expiration: Expiration,
        custodians: &[PublicKey],
        req_confirms: u8,
        expiration_time: Option<u32>,
    ) -> Result<Box<dyn UnsignedMessage>> {
        match self.wallet_type {
            WalletType::Multisig(multisig_type) => multisig::prepare_deploy(
                self.clock.as_ref(),
                &self.public_key,
                multisig_type,
                self.workchain(),
                expiration,
                multisig::DeployParams {
                    owners: custodians,
                    req_confirms,
                    expiration_time,
                },
            ),
            // Non-multisig wallets doesn't support multiple owners
            _ => Err(TonWalletError::InvalidContractType.into()),
        }
    }

    pub fn make_unsigned_wallet_v5_transfer_payload(
        &self,
        current_state: &ton_block::AccountStuff,
        public_key: &PublicKey,
        gifts: Vec<Gift>,
        expiration: Expiration,
        is_internal_flow: bool,
    ) -> Result<(UInt256, BuilderData)> {
        if !matches!(self.wallet_type, WalletType::WalletV5R1) {
            return Err(TonWalletError::InvalidContractType.into());
        }
        let (init_data, _) = wallet_v5r1::get_init_data(current_state.storage.state(), public_key)?;

        let expire_at = ExpireAt::new(self.clock.as_ref(), expiration);
        let (hash, builder_data) =
            init_data.make_transfer_payload(gifts, expire_at.timestamp, is_internal_flow)?;
        Ok((hash, builder_data))
    }

    pub fn make_unsigned_new_wallet_v5_transfer_payload(
        &self,
        state_init: &ton_block::StateInit,
        gifts: Vec<Gift>,
        expiration: Expiration,
        is_internal_flow: bool,
    ) -> Result<(UInt256, BuilderData)> {
        if !matches!(self.wallet_type, WalletType::WalletV5R1) {
            return Err(TonWalletError::InvalidContractType.into());
        }
        let init_data = wallet_v5r1::get_init_data_from_state_init(state_init)?;

        let expire_at = ExpireAt::new(self.clock.as_ref(), expiration);
        let (hash, builder_data) =
            init_data.make_transfer_payload(gifts, expire_at.timestamp, is_internal_flow)?;
        Ok((hash, builder_data))
    }

    pub fn get_wallet_v5_seqno(
        &self,
        current_state: &ton_block::AccountStuff,
        public_key: &PublicKey,
    ) -> Result<u32> {
        if !matches!(self.wallet_type, WalletType::WalletV5R1) {
            return Err(TonWalletError::InvalidContractType.into());
        }

        let (init_data, _) = wallet_v5r1::get_init_data(current_state.storage.state(), public_key)?;
        Ok(init_data.seqno)
    }

    pub fn prepare_transfer(
        &mut self,
        current_state: &ton_block::AccountStuff,
        public_key: &PublicKey,
        gifts: Vec<Gift>,
        expiration: Expiration,
    ) -> Result<TransferAction> {
        match self.wallet_type {
            WalletType::Multisig(multisig_type) => {
                anyhow::ensure!(
                    gifts.len() == 1,
                    "Multiple outgoing messages are not supported by multisig contract"
                );
                let gift = gifts.into_iter().next().unwrap();

                match &current_state.storage.state {
                    ton_block::AccountState::AccountFrozen { .. } => {
                        return Err(TonWalletError::AccountIsFrozen.into())
                    }
                    ton_block::AccountState::AccountUninit => {
                        return Ok(TransferAction::DeployFirst)
                    }
                    ton_block::AccountState::AccountActive { .. } => {}
                };

                self.wallet_data.update(
                    self.clock.as_ref(),
                    &self.public_key,
                    self.wallet_type,
                    current_state,
                    self.handler.as_ref(),
                )?;

                let has_multiple_owners = match &self.wallet_data.custodians {
                    Some(custodians) => custodians.len() > 1,
                    None => return Err(TonWalletError::CustodiansNotFound.into()),
                };

                multisig::prepare_transfer(
                    self.clock.as_ref(),
                    multisig_type,
                    public_key,
                    has_multiple_owners,
                    self.address().clone(),
                    gift,
                    expiration,
                )
            }
            WalletType::WalletV3 => wallet_v3::prepare_transfer(
                self.clock.as_ref(),
                public_key,
                current_state,
                0,
                gifts,
                expiration,
            ),
            WalletType::WalletV3R1 => wallet_v3v4::prepare_transfer(
                self.clock.as_ref(),
                public_key,
                current_state,
                0,
                gifts,
                expiration,
                wallet_v3v4::WalletVersion::V3R1,
            ),
            WalletType::WalletV3R2 => wallet_v3v4::prepare_transfer(
                self.clock.as_ref(),
                public_key,
                current_state,
                0,
                gifts,
                expiration,
                wallet_v3v4::WalletVersion::V3R2,
            ),
            WalletType::WalletV4R1 => wallet_v3v4::prepare_transfer(
                self.clock.as_ref(),
                public_key,
                current_state,
                0,
                gifts,
                expiration,
                wallet_v3v4::WalletVersion::V4R1,
            ),
            WalletType::WalletV4R2 => wallet_v3v4::prepare_transfer(
                self.clock.as_ref(),
                public_key,
                current_state,
                0,
                gifts,
                expiration,
                wallet_v3v4::WalletVersion::V4R2,
            ),
            WalletType::WalletV5R1 => wallet_v5r1::prepare_transfer(
                self.clock.as_ref(),
                public_key,
                current_state,
                0,
                gifts,
                expiration,
            ),
            WalletType::EverWallet => ever_wallet::prepare_transfer(
                self.clock.as_ref(),
                public_key,
                current_state,
                self.address().clone(),
                gifts,
                expiration,
            ),
            WalletType::HighloadWalletV2 => highload_wallet_v2::prepare_transfer(
                self.clock.as_ref(),
                public_key,
                current_state,
                gifts,
                expiration,
            ),
        }
    }

    pub fn prepare_confirm_transaction(
        &self,
        current_state: &ton_block::AccountStuff,
        public_key: &PublicKey,
        transaction_id: u64,
        expiration: Expiration,
    ) -> Result<Box<dyn UnsignedMessage>> {
        match self.wallet_type {
            WalletType::Multisig(multisig_type) => {
                let has_pending_transaction = multisig::find_pending_transaction(
                    self.clock.as_ref(),
                    multisig_type,
                    Cow::Borrowed(current_state),
                    transaction_id,
                )?;
                if !has_pending_transaction {
                    return Err(TonWalletError::PendingTransactionNotFound.into());
                }

                multisig::prepare_confirm_transaction(
                    self.clock.as_ref(),
                    multisig_type,
                    public_key,
                    self.address().clone(),
                    transaction_id,
                    expiration,
                )
            }
            // Non-multisig wallets doesn't support pending transactions
            _ => Err(TonWalletError::PendingTransactionNotFound.into()),
        }
    }

    pub fn prepare_code_update(
        &self,
        public_key: &PublicKey,
        new_code_hash: &[u8; 32],
        expiration: Expiration,
    ) -> Result<Box<dyn UnsignedMessage>> {
        match self.wallet_type {
            WalletType::Multisig(multisig_type) if multisig_type.is_multisig2() => {
                multisig::prepare_code_update(
                    self.clock.as_ref(),
                    multisig_type,
                    public_key,
                    self.address().clone(),
                    new_code_hash,
                    expiration,
                )
            }
            _ => Err(TonWalletError::UpdateNotSupported.into()),
        }
    }

    pub fn prepare_confirm_update(
        &self,
        current_state: &ton_block::AccountStuff,
        public_key: &PublicKey,
        update_id: u64,
        expiration: Expiration,
    ) -> Result<Box<dyn UnsignedMessage>> {
        match self.wallet_type {
            WalletType::Multisig(multisig_type) => {
                let pending_update = multisig::find_pending_update(
                    self.clock.as_ref(),
                    multisig_type,
                    Cow::Borrowed(current_state),
                    update_id,
                )?;
                if pending_update.is_none() {
                    return Err(TonWalletError::PendingUpdateNotFound.into());
                }

                multisig::prepare_confirm_update(
                    self.clock.as_ref(),
                    multisig_type,
                    public_key,
                    self.address().clone(),
                    update_id,
                    expiration,
                )
            }
            _ => Err(TonWalletError::PendingUpdateNotFound.into()),
        }
    }

    pub fn prepare_execute_code_update(
        &self,
        current_state: &ton_block::AccountStuff,
        public_key: &PublicKey,
        update_id: u64,
        new_code: ton_types::Cell,
        expiration: Expiration,
    ) -> Result<Box<dyn UnsignedMessage>> {
        match self.wallet_type {
            WalletType::Multisig(multisig_type) => {
                let update = match multisig::find_pending_update(
                    self.clock.as_ref(),
                    multisig_type,
                    Cow::Borrowed(current_state),
                    update_id,
                )? {
                    Some(update) => update,
                    None => return Err(TonWalletError::PendingUpdateNotFound.into()),
                };

                if !matches!(update.new_code_hash, Some(hash) if new_code.repr_hash() == hash) {
                    return Err(TonWalletError::UpdatedDataMismatch.into());
                }

                multisig::prepare_execute_update(
                    self.clock.as_ref(),
                    multisig_type,
                    public_key,
                    self.address().clone(),
                    update_id,
                    Some(new_code),
                    expiration,
                )
            }
            _ => Err(TonWalletError::PendingUpdateNotFound.into()),
        }
    }

    pub async fn send(
        &mut self,
        message: &ton_block::Message,
        expire_at: u32,
    ) -> Result<PendingTransaction> {
        self.contract_subscription.send(message, expire_at).await
    }

    pub async fn refresh(&mut self) -> Result<()> {
        let handler = self.handler.as_ref();
        self.contract_subscription
            .refresh(
                &mut make_contract_state_handler(
                    self.clock.as_ref(),
                    handler,
                    &self.public_key,
                    self.wallet_type,
                    &mut self.wallet_data,
                ),
                &mut make_transactions_handler(handler, self.wallet_type),
                &mut make_message_sent_handler(handler),
                &mut make_message_expired_handler(handler),
            )
            .await
    }

    pub async fn handle_block(&mut self, block: &ton_block::Block) -> Result<()> {
        // TODO: update wallet data here

        let handler = self.handler.as_ref();
        let new_account_state = self.contract_subscription.handle_block(
            block,
            &mut make_transactions_handler(handler, self.wallet_type),
            &mut make_message_sent_handler(handler),
            &mut make_message_expired_handler(handler),
        )?;

        if let Some(account_state) = new_account_state {
            handler.on_state_changed(account_state);
        }

        Ok(())
    }

    pub async fn preload_transactions(&mut self, from_lt: u64) -> Result<()> {
        let handler = self.handler.as_ref();
        self.contract_subscription
            .preload_transactions(
                from_lt,
                &mut make_transactions_handler(handler, self.wallet_type),
            )
            .await
    }

    pub async fn estimate_fees(&mut self, message: &ton_block::Message) -> Result<u128> {
        self.contract_subscription.estimate_fees(message).await
    }
}

#[derive(Default)]
struct WalletData {
    custodians: Option<Vec<UInt256>>,
    unconfirmed_transactions: Vec<MultisigPendingTransaction>,
    unconfirmed_updates: Vec<MultisigPendingUpdate>,
    details: Option<TonWalletDetails>,
}

impl WalletData {
    fn update(
        &mut self,
        clock: &dyn Clock,
        public_key: &PublicKey,
        wallet_type: WalletType,
        account_stuff: &ton_block::AccountStuff,
        handler: &dyn TonWalletSubscriptionHandler,
    ) -> Result<()> {
        // Extract details
        if self.details.is_none() {
            let mut details = wallet_type.details();

            if let WalletType::Multisig(multisig_type) = wallet_type {
                let params =
                    multisig::get_params(clock, multisig_type, Cow::Borrowed(account_stuff))?;
                details.expiration_time = params.expiration_time.try_into().unwrap_or(u32::MAX);
                details.required_confirmations =
                    NonZeroU8::new(std::cmp::max(params.required_confirms, 1));
            }

            self.details = Some(details);
            handler.on_details_changed(details);
        }

        // Extract custodians
        let multisig_type = match wallet_type {
            WalletType::Multisig(multisig_type) => multisig_type,
            // Simple path for wallets with single custodian
            _ => {
                if self.custodians.is_none() {
                    let custodians = self.custodians.insert(vec![public_key.to_bytes().into()]);
                    handler.on_custodians_changed(custodians);
                }
                return Ok(());
            }
        };

        // Extract custodians
        let custodians = match &mut self.custodians {
            Some(custodians) => custodians,
            None => {
                let custodians = self.custodians.insert(multisig::get_custodians(
                    clock,
                    multisig_type,
                    Cow::Borrowed(account_stuff),
                )?);
                handler.on_custodians_changed(custodians);
                custodians
            }
        };

        if multisig_type.is_updatable() {
            let pending_updates = multisig::get_pending_updates(
                clock,
                multisig_type,
                Cow::Borrowed(account_stuff),
                custodians,
            )?;
            if self.unconfirmed_updates != pending_updates {
                self.unconfirmed_updates = pending_updates;
                handler.on_unconfirmed_updates_changed(&self.unconfirmed_updates);
            }
        }

        // Skip pending transactions extraction for single custodian
        if custodians.len() < 2 {
            return Ok(());
        }

        // Extract pending transactions
        let pending_transactions = multisig::get_pending_transactions(
            clock,
            multisig_type,
            Cow::Borrowed(account_stuff),
            custodians,
        )?;

        if self.unconfirmed_transactions != pending_transactions {
            self.unconfirmed_transactions = pending_transactions;
            handler.on_unconfirmed_transactions_changed(&self.unconfirmed_transactions);
        }

        Ok(())
    }
}

pub fn extract_wallet_init_data(contract: &ExistingContract) -> Result<(PublicKey, WalletType)> {
    let (code, data) = match &contract.account.storage.state {
        ton_block::AccountState::AccountActive {
            state_init:
                ton_block::StateInit {
                    code: Some(code),
                    data: Some(data),
                    ..
                },
            ..
        } => (code, data),
        _ => return Err(TonWalletError::AccountNotExists.into()),
    };

    let code_hash = code.repr_hash();
    if let Some(multisig_type) = multisig::guess_multisig_type(&code_hash) {
        let public_key = extract_public_key(&contract.account)?;
        Ok((public_key, WalletType::Multisig(multisig_type)))
    } else if wallet_v3::is_wallet_v3(&code_hash) {
        let public_key = PublicKey::from_bytes(wallet_v3::InitData::try_from(data)?.public_key())?;
        Ok((public_key, WalletType::WalletV3))
    } else if wallet_v3v4::is_wallet_v4r1(&code_hash) {
        let public_key =
            PublicKey::from_bytes(wallet_v3v4::InitData::try_from(data)?.public_key())?;
        Ok((public_key, WalletType::WalletV4R1))
    } else if wallet_v3v4::is_wallet_v4r2(&code_hash) {
        let public_key =
            PublicKey::from_bytes(wallet_v3v4::InitData::try_from(data)?.public_key())?;
        Ok((public_key, WalletType::WalletV4R2))
    } else if wallet_v5r1::is_wallet_v5r1(&code_hash) {
        let public_key =
            PublicKey::from_bytes(wallet_v5r1::InitData::try_from(data)?.public_key())?;
        Ok((public_key, WalletType::WalletV5R1))
    } else if ever_wallet::is_ever_wallet(&code_hash) {
        let public_key = extract_public_key(&contract.account)?;
        Ok((public_key, WalletType::EverWallet))
    } else if highload_wallet_v2::is_highload_wallet_v2(&code_hash) {
        let public_key =
            PublicKey::from_bytes(highload_wallet_v2::InitData::try_from(data)?.public_key())?;
        Ok((public_key, WalletType::HighloadWalletV2))
    } else {
        Err(TonWalletError::InvalidContractType.into())
    }
}

pub fn get_wallet_custodians(
    clock: &dyn Clock,
    contract: &ExistingContract,
    public_key: &PublicKey,
    wallet_type: WalletType,
) -> Result<Vec<UInt256>> {
    match wallet_type {
        WalletType::Multisig(multisig_type) => {
            multisig::get_custodians(clock, multisig_type, Cow::Borrowed(&contract.account))
        }
        _ => Ok(vec![public_key.to_bytes().into()]),
    }
}

pub const WALLET_TYPES_BY_POPULARITY: [WalletType; 10] = [
    WalletType::Multisig(MultisigType::SafeMultisigWallet),
    WalletType::Multisig(MultisigType::SurfWallet),
    WalletType::WalletV3,
    WalletType::EverWallet,
    WalletType::Multisig(MultisigType::Multisig2_1),
    WalletType::Multisig(MultisigType::Multisig2),
    WalletType::Multisig(MultisigType::SetcodeMultisigWallet),
    WalletType::Multisig(MultisigType::SafeMultisigWallet24h),
    WalletType::Multisig(MultisigType::BridgeMultisigWallet),
    WalletType::HighloadWalletV2,
];

pub async fn find_existing_wallets(
    transport: &dyn Transport,
    public_key: &PublicKey,
    workchain_id: i8,
    wallet_types: &[WalletType],
) -> Result<Vec<ExistingWalletInfo>> {
    use futures_util::stream::{FuturesUnordered, TryStreamExt};

    wallet_types
        .iter()
        .map(|&wallet_type| async move {
            let address = compute_address(public_key, wallet_type, workchain_id);

            let contract_state = transport.get_contract_state(&address).await?;

            Ok(ExistingWalletInfo {
                address,
                public_key: *public_key,
                wallet_type,
                contract_state: contract_state.brief(),
            })
        })
        .collect::<FuturesUnordered<_>>()
        .try_collect::<Vec<ExistingWalletInfo>>()
        .await
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExistingWalletInfo {
    #[serde(with = "serde_address")]
    pub address: MsgAddressInt,
    #[serde(with = "serde_public_key")]
    pub public_key: PublicKey,
    pub wallet_type: WalletType,
    pub contract_state: ContractState,
}

pub trait InternalMessageSender {
    fn prepare_transfer(
        &mut self,
        current_state: &ton_block::AccountStuff,
        public_key: &PublicKey,
        message: InternalMessage,
        expiration: Expiration,
    ) -> Result<TransferAction>;
}

impl InternalMessageSender for TonWallet {
    fn prepare_transfer(
        &mut self,
        current_state: &ton_block::AccountStuff,
        public_key: &PublicKey,
        message: InternalMessage,
        expiration: Expiration,
    ) -> Result<TransferAction> {
        if matches!(message.source, Some(source) if &source != self.address()) {
            return Err(InternalMessageSenderError::InvalidSender.into());
        }

        let gift = Gift {
            flags: MessageFlags::default().into(),
            bounce: message.bounce,
            destination: message.destination,
            amount: message.amount,
            body: Some(message.body),
            state_init: None,
        };

        self.prepare_transfer(current_state, public_key, vec![gift], expiration)
    }
}

#[derive(thiserror::Error, Debug)]
enum InternalMessageSenderError {
    #[error("Invalid sender")]
    InvalidSender,
}

#[derive(thiserror::Error, Debug)]
enum TonWalletError {
    #[error("Account not exists")]
    AccountNotExists,
    #[error("Account is frozen")]
    AccountIsFrozen,
    #[error("Invalid contract type")]
    InvalidContractType,
    #[error("Custodians not found")]
    CustodiansNotFound,
    #[error("Pending transaction not found")]
    PendingTransactionNotFound,
    #[error("Update not supported")]
    UpdateNotSupported,
    #[error("Pending update not found")]
    PendingUpdateNotFound,
    #[error("Updated data mismatch")]
    UpdatedDataMismatch,
}

fn make_contract_state_handler<'a>(
    clock: &'a dyn Clock,
    handler: &'a dyn TonWalletSubscriptionHandler,
    public_key: &'a PublicKey,
    wallet_type: WalletType,
    wallet_data: &'a mut WalletData,
) -> impl FnMut(&RawContractState) + 'a {
    move |contract_state| {
        if let RawContractState::Exists(contract_state) = contract_state {
            if let Err(e) = wallet_data.update(
                clock,
                public_key,
                wallet_type,
                &contract_state.account,
                handler,
            ) {
                log::error!("{e}");
            }
        }
        handler.on_state_changed(contract_state.brief())
    }
}

fn make_transactions_handler(
    handler: &'_ dyn TonWalletSubscriptionHandler,
    wallet_type: WalletType,
) -> impl FnMut(Vec<RawTransaction>, TransactionsBatchInfo) + '_ {
    move |transactions, batch_info| {
        let transactions = transactions
            .into_iter()
            .filter_map(move |transaction| {
                let data = parse_transaction_additional_info(&transaction.data, wallet_type);
                let transaction =
                    Transaction::try_from((transaction.hash, transaction.data)).ok()?;
                Some(TransactionWithData { transaction, data })
            })
            .collect();

        handler.on_transactions_found(transactions, batch_info)
    }
}

fn make_message_sent_handler(
    handler: &'_ dyn TonWalletSubscriptionHandler,
) -> impl FnMut(PendingTransaction, RawTransaction) + '_ {
    move |pending_transaction, transaction| {
        let transaction = Transaction::try_from((transaction.hash, transaction.data)).ok();
        handler.on_message_sent(pending_transaction, transaction);
    }
}

fn make_message_expired_handler(
    handler: &'_ dyn TonWalletSubscriptionHandler,
) -> impl FnMut(PendingTransaction) + '_ {
    move |pending_transaction| handler.on_message_expired(pending_transaction)
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TonWalletDetails {
    pub requires_separate_deploy: bool,
    #[serde(with = "serde_string")]
    pub min_amount: u64,
    pub max_messages: usize,
    pub supports_payload: bool,
    pub supports_state_init: bool,
    pub supports_multiple_owners: bool,
    pub supports_code_update: bool,
    pub expiration_time: u32,
    pub required_confirmations: Option<NonZeroU8>,
}

/// Message info
#[derive(Clone)]
pub struct Gift {
    pub flags: u8,
    pub bounce: bool,
    pub destination: IntAddr,
    pub amount: u128,
    pub body: Option<Cell>,
    pub state_init: Option<StateInit>,
}

#[derive(Clone)]
pub enum TransferAction {
    DeployFirst,
    Sign(Box<dyn UnsignedMessage>),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum WalletType {
    Multisig(MultisigType),
    WalletV3,
    WalletV3R1,
    WalletV3R2,
    WalletV4R1,
    WalletV4R2,
    WalletV5R1,
    HighloadWalletV2,
    EverWallet,
}

impl WalletType {
    pub fn details(&self) -> TonWalletDetails {
        match self {
            Self::Multisig(multisig_type) => multisig::ton_wallet_details(*multisig_type),
            Self::WalletV3 => wallet_v3::DETAILS,
            Self::WalletV5R1 => wallet_v5r1::DETAILS,
            Self::HighloadWalletV2 => highload_wallet_v2::DETAILS,
            Self::EverWallet => ever_wallet::DETAILS,
            Self::WalletV3R1 | Self::WalletV3R2 | Self::WalletV4R1 | Self::WalletV4R2 => {
                wallet_v3v4::DETAILS
            }
        }
    }

    pub fn possible_updates(&self) -> &'static [Self] {
        const MULTISIG2_UPDATES: &[WalletType] = &[WalletType::Multisig(MultisigType::Multisig2_1)];

        match self {
            Self::Multisig(MultisigType::Multisig2) => MULTISIG2_UPDATES,
            _ => &[],
        }
    }

    pub fn code_hash(&self) -> &[u8; 32] {
        match self {
            Self::Multisig(multisig_type) => multisig_type.code_hash(),
            Self::WalletV3 => wallet_v3::CODE_HASH,
            Self::WalletV3R1 => wallet_v3v4::CODE_HASH_V3_R1,
            Self::WalletV3R2 => wallet_v3v4::CODE_HASH_V3_R2,
            Self::WalletV4R1 => wallet_v3v4::CODE_HASH_V4_R1,
            Self::WalletV4R2 => wallet_v3v4::CODE_HASH_V4_R2,
            Self::WalletV5R1 => wallet_v5r1::CODE_HASH,
            Self::HighloadWalletV2 => highload_wallet_v2::CODE_HASH,
            Self::EverWallet => ever_wallet::CODE_HASH,
        }
    }

    pub fn code(&self) -> ton_types::Cell {
        use nekoton_contracts::wallets;
        match self {
            Self::Multisig(multisig_type) => multisig_type.code(),
            Self::WalletV3 => wallets::code::wallet_v3(),
            Self::WalletV3R1 => wallets::code::wallet_v3r1(),
            Self::WalletV3R2 => wallets::code::wallet_v3r2(),
            Self::WalletV4R1 => wallets::code::wallet_v4r1(),
            Self::WalletV4R2 => wallets::code::wallet_v4r2(),
            Self::WalletV5R1 => wallets::code::wallet_v5r1(),
            Self::HighloadWalletV2 => wallets::code::highload_wallet_v2(),
            Self::EverWallet => wallets::code::ever_wallet(),
        }
    }
}

impl FromStr for WalletType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "WalletV3" => Self::WalletV3,
            "WalletV3R1" => Self::WalletV3R1,
            "WalletV3R2" => Self::WalletV3R2,
            "WalletV4R1" => Self::WalletV4R1,
            "WalletV4R2" => Self::WalletV4R2,
            "WalletV5R1" => Self::WalletV5R1,
            "HighloadWalletV2" => Self::HighloadWalletV2,
            "EverWallet" => Self::EverWallet,
            s => Self::Multisig(MultisigType::from_str(s)?),
        })
    }
}

impl TryInto<u16> for WalletType {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<u16, Self::Error> {
        let res = match self {
            WalletType::WalletV3 => 0,
            WalletType::EverWallet => 1,
            WalletType::Multisig(MultisigType::SafeMultisigWallet) => 2,
            WalletType::Multisig(MultisigType::SafeMultisigWallet24h) => 3,
            WalletType::Multisig(MultisigType::SetcodeMultisigWallet) => 4,
            WalletType::Multisig(MultisigType::BridgeMultisigWallet) => 5,
            WalletType::Multisig(MultisigType::SurfWallet) => 6,
            WalletType::Multisig(MultisigType::Multisig2) => 7,
            WalletType::Multisig(MultisigType::Multisig2_1) => 8,
            WalletType::WalletV4R1 => 9,
            WalletType::WalletV4R2 => 10,
            WalletType::WalletV5R1 => 11,
            _ => anyhow::bail!("Unimplemented wallet type"),
        };

        Ok(res)
    }
}

impl std::fmt::Display for WalletType {
    fn fmt(&self, f: &'_ mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Multisig(multisig_type) => multisig_type.fmt(f),
            Self::WalletV3 => f.write_str("WalletV3"),
            Self::WalletV3R1 => f.write_str("WalletV3R1"),
            Self::WalletV3R2 => f.write_str("WalletV3R2"),
            Self::WalletV4R1 => f.write_str("WalletV4R1"),
            Self::WalletV4R2 => f.write_str("WalletV4R2"),
            Self::WalletV5R1 => f.write_str("WalletV5R1"),
            Self::HighloadWalletV2 => f.write_str("HighloadWalletV2"),
            Self::EverWallet => f.write_str("EverWallet"),
        }
    }
}

pub fn compute_address(
    public_key: &PublicKey,
    wallet_type: WalletType,
    workchain_id: i8,
) -> MsgAddressInt {
    match wallet_type {
        WalletType::Multisig(multisig_type) => {
            multisig::compute_contract_address(public_key, multisig_type, workchain_id)
        }
        WalletType::WalletV3 => wallet_v3::compute_contract_address(public_key, workchain_id),
        WalletType::WalletV5R1 => wallet_v5r1::compute_contract_address(public_key, workchain_id),
        WalletType::EverWallet => ever_wallet::compute_contract_address(public_key, workchain_id),
        WalletType::HighloadWalletV2 => {
            highload_wallet_v2::compute_contract_address(public_key, workchain_id)
        }
        WalletType::WalletV3R1 => wallet_v3v4::compute_contract_address(
            public_key,
            workchain_id,
            wallet_v3v4::WalletVersion::V3R1,
        ),
        WalletType::WalletV3R2 => wallet_v3v4::compute_contract_address(
            public_key,
            workchain_id,
            wallet_v3v4::WalletVersion::V3R2,
        ),
        WalletType::WalletV4R1 => wallet_v3v4::compute_contract_address(
            public_key,
            workchain_id,
            wallet_v3v4::WalletVersion::V4R1,
        ),
        WalletType::WalletV4R2 => wallet_v3v4::compute_contract_address(
            public_key,
            workchain_id,
            wallet_v3v4::WalletVersion::V4R2,
        ),
    }
}

pub trait TonWalletSubscriptionHandler: Send + Sync {
    /// Called when found transaction which is relative with one of the pending transactions
    fn on_message_sent(
        &self,
        pending_transaction: PendingTransaction,
        transaction: Option<Transaction>,
    );

    /// Called when no transactions produced for the specific message before some expiration time
    fn on_message_expired(&self, pending_transaction: PendingTransaction);

    /// Called every time a new state is detected
    fn on_state_changed(&self, new_state: ContractState) {
        let _ = new_state;
    }

    /// Called every time new transactions are detected.
    /// - When new block found
    /// - When manually requesting the latest transactions (can be called several times)
    /// - When preloading transactions
    fn on_transactions_found(
        &self,
        transactions: Vec<TransactionWithData<TransactionAdditionalInfo>>,
        batch_info: TransactionsBatchInfo,
    ) {
        let _ = transactions;
        let _ = batch_info;
    }

    /// Called when wallet details changed (e.g. expiration time or required confirms)
    fn on_details_changed(&self, details: TonWalletDetails) {
        let _ = details;
    }

    /// Called when wallet custodians changed (e.g. on code upgrade or first refresh)
    fn on_custodians_changed(&self, custodians: &[UInt256]) {
        let _ = custodians;
    }

    /// Called when wallet has new pending transactions set
    fn on_unconfirmed_transactions_changed(
        &self,
        unconfirmed_transactions: &[MultisigPendingTransaction],
    ) {
        let _ = unconfirmed_transactions;
    }

    /// Called when wallet has new pending updates set
    fn on_unconfirmed_updates_changed(&self, unconfirmed_updates: &[MultisigPendingUpdate]) {
        let _ = unconfirmed_updates;
    }
}
