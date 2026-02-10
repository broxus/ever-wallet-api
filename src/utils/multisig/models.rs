use anyhow::{anyhow, Result};
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use tycho_types::{
    abi::{AbiValue, FromAbi, IntoAbi, NamedAbiValue},
    cell::{Cell, HashBytes},
    models::StdAddr,
};

use crate::utils::{
    serde_address, serde_cell, serde_string, ton_wallet::multisig::UnpackerError,
    wallets::multisig2, ContractCall, FromAbiPlain, InputMessage, IntoAbiPlain,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "data")]
pub enum MultisigTransaction {
    Send(MultisigSendTransaction),
    Submit(MultisigSubmitTransaction),
    Confirm(MultisigConfirmTransaction),
    SubmitUpdate(MultisigSubmitUpdate),
    ConfirmUpdate(MultisigConfirmUpdate),
    ExecuteUpdate(MultisigExecuteUpdate),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy, IntoAbi, FromAbi)]
#[serde(rename_all = "camelCase")]
pub struct MultisigConfirmTransaction {
    pub custodian: HashBytes,

    #[serde(with = "serde_string")]
    pub transaction_id: u64,
}

impl IntoAbiPlain for MultisigConfirmTransaction {
    fn into_abi_plain(self) -> Vec<NamedAbiValue> {
        vec![AbiValue::Uint(64, self.transaction_id.into()).named("transactionId")]
    }
    fn as_abi_plain(&self) -> Vec<NamedAbiValue> {
        vec![AbiValue::Uint(64, self.transaction_id.into()).named("transactionId")]
    }
}
impl FromAbiPlain for MultisigConfirmTransaction {
    fn from_abi_plain(value: Vec<NamedAbiValue>) -> anyhow::Result<Self> {
        let AbiValue::Uint(_, transaction_id) = &value[0].value else {
            return Err(anyhow!("Invalid transactionId"));
        };

        Ok(MultisigConfirmTransaction {
            custodian: Default::default(),
            transaction_id: transaction_id
                .to_u64()
                .ok_or(anyhow!("Invalid transaction id"))?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, IntoAbi, FromAbi)]
#[serde(rename_all = "camelCase")]
pub struct MultisigSubmitTransaction {
    #[serde(with = "serde_string")]
    pub custodian: HashBytes,

    #[serde(with = "serde_address")]
    pub dest: StdAddr,

    #[serde(with = "serde_string")]
    pub value: u128,

    pub bounce: bool,

    pub all_balance: bool,

    #[serde(with = "serde_cell")]
    pub payload: Cell,

    #[serde(with = "serde_string")]
    pub trans_id: u64,
}
impl IntoAbiPlain for MultisigSubmitTransaction {}
impl FromAbiPlain for MultisigSubmitTransaction {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, IntoAbi, FromAbi)]
pub struct MultisigSendTransaction {
    #[serde(with = "serde_address")]
    pub dest: StdAddr,

    #[serde(with = "serde_string")]
    pub value: u128,

    pub bounce: bool,

    pub flags: u8,

    #[serde(with = "serde_cell")]
    pub payload: Cell,
}
impl IntoAbiPlain for MultisigSendTransaction {}
impl FromAbiPlain for MultisigSendTransaction {}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MultisigSubmitUpdate {
    pub custodian: HashBytes,
    pub new_code_hash: Option<HashBytes>,
    pub new_owners: bool,
    pub new_req_confirms: bool,
    pub new_lifetime: bool,
    #[serde(with = "serde_string")]
    pub update_id: u64,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MultisigConfirmUpdate {
    pub custodian: HashBytes,
    #[serde(with = "serde_string")]
    pub update_id: u64,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MultisigExecuteUpdate {
    pub custodian: HashBytes,
    #[serde(with = "serde_string")]
    pub update_id: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct MultisigPendingTransaction {
    #[serde(with = "serde_string")]
    pub id: u64,

    pub confirmations: Vec<HashBytes>,

    pub signs_required: u8,
    pub signs_received: u8,

    #[serde(with = "serde_string")]
    pub creator: HashBytes,

    pub index: u8,

    #[serde(with = "serde_address")]
    pub dest: StdAddr,

    #[serde(with = "serde_string")]
    pub value: u128,

    pub send_flags: u16,

    #[serde(with = "serde_cell")]
    pub payload: Cell,

    pub bounce: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct MultisigPendingUpdate {
    #[serde(with = "serde_string")]
    pub id: u64,

    pub confirmations: Vec<HashBytes>,

    pub signs_received: u8,

    #[serde(with = "serde_string")]
    pub creator: HashBytes,

    pub index: u8,

    pub new_code_hash: Option<HashBytes>,
    pub new_custodians: Option<Vec<HashBytes>>,
    pub new_req_confirms: Option<u8>,
    pub new_lifetime: Option<u32>,
}

impl TryFrom<(HashBytes, InputMessage)> for MultisigConfirmTransaction {
    type Error = UnpackerError;

    fn try_from((custodian, value): (HashBytes, InputMessage)) -> Result<Self, Self::Error> {
        let output = MultisigConfirmTransaction::from_abi_plain(value.0)
            .map_err(|_| UnpackerError::InvalidAbi)?;
        Ok(Self {
            custodian,
            transaction_id: output.transaction_id,
        })
    }
}

#[derive(Clone, Debug, FromAbi)]
struct MultisigSubmitTransactionInput {
    dest: StdAddr,
    value: u128,
    bounce: bool,
    all_balance: bool,
    payload: Cell,
}

impl FromAbiPlain for MultisigSubmitTransactionInput {}

#[derive(Clone, Debug, FromAbi)]
struct MultisigSubmitTransactionOutput {
    trans_id: u64,
}

impl FromAbiPlain for MultisigSubmitTransactionOutput {}

impl TryFrom<(HashBytes, ContractCall)> for MultisigSubmitTransaction {
    type Error = UnpackerError;

    fn try_from((custodian, value): (HashBytes, ContractCall)) -> Result<Self, Self::Error> {
        let input = MultisigSubmitTransactionInput::from_abi_plain(value.inputs)
            .map_err(|_| UnpackerError::InvalidAbi)?;
        let output = MultisigSubmitTransactionOutput::from_abi_plain(value.outputs)
            .map_err(|_| UnpackerError::InvalidAbi)?;

        Ok(Self {
            custodian,
            dest: input.dest,
            value: input.value,
            bounce: input.bounce,
            all_balance: input.all_balance,
            payload: input.payload,
            trans_id: output.trans_id,
        })
    }
}

impl TryFrom<InputMessage> for MultisigSendTransaction {
    type Error = UnpackerError;

    fn try_from(value: InputMessage) -> Result<Self, Self::Error> {
        let input = MultisigSendTransaction::from_abi_plain(value.0)
            .map_err(|_| UnpackerError::InvalidAbi)?;

        Ok(Self {
            dest: input.dest,
            value: input.value,
            bounce: input.bounce,
            flags: input.flags,
            payload: input.payload,
        })
    }
}

impl TryFrom<(HashBytes, ContractCall)> for MultisigSubmitUpdate {
    type Error = UnpackerError;

    fn try_from((custodian, value): (HashBytes, ContractCall)) -> Result<Self, Self::Error> {
        let input = multisig2::SubmitUpdateParams::from_abi_plain(value.inputs)
            .map_err(|_| UnpackerError::InvalidAbi)?;
        let output = multisig2::SubmitUpdateOutput::from_abi_plain(value.outputs)
            .map_err(|_| UnpackerError::InvalidAbi)?;

        Ok(Self {
            custodian,
            new_code_hash: input.code_hash,
            new_owners: input.owners.is_some(),
            new_req_confirms: input.req_confirms.is_some(),
            new_lifetime: input.lifetime.is_some(),
            update_id: output.update_id,
        })
    }
}

impl TryFrom<(HashBytes, InputMessage)> for MultisigConfirmUpdate {
    type Error = UnpackerError;

    fn try_from((custodian, input): (HashBytes, InputMessage)) -> Result<Self, Self::Error> {
        let input = multisig2::ConfirmUpdateParams::from_abi_plain(input.0)
            .map_err(|_| UnpackerError::InvalidAbi)?;
        Ok(Self {
            custodian,
            update_id: input.update_id,
        })
    }
}

impl TryFrom<(HashBytes, InputMessage)> for MultisigExecuteUpdate {
    type Error = UnpackerError;

    fn try_from((custodian, input): (HashBytes, InputMessage)) -> Result<Self, Self::Error> {
        let input = multisig2::ExecuteUpdateParams::from_abi_plain(input.0)
            .map_err(|_| UnpackerError::InvalidAbi)?;
        Ok(Self {
            custodian,
            update_id: input.update_id,
        })
    }
}
