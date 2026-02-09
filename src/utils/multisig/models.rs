use anyhow::{anyhow, Result};
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use tycho_types::{
    abi::{AbiType, AbiValue, NamedAbiValue},
    cell::{Cell, HashBytes},
    models::{AnyAddr, StdAddr},
};

use crate::utils::{
    serde_address, serde_cell, serde_string, ton_wallet::multisig::UnpackerError,
    wallets::multisig2, ContractCall, InputMessage,
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
#[serde(rename_all = "camelCase")]
pub struct MultisigConfirmTransaction {
    pub custodian: HashBytes,

    #[serde(with = "serde_string")]
    pub transaction_id: u64,
}

impl MultisigConfirmTransaction {
    pub fn unpack(values: Vec<NamedAbiValue>) -> Result<Self> {
        if values.len() != 1 {
            return Err(anyhow!("Invalid number of arguments"));
        }

        let transaction_id_abi_value = &values[0];
        if &*transaction_id_abi_value.name != "transactionId"
            && transaction_id_abi_value.value.get_type() != AbiType::Uint(64)
        {
            return Err(anyhow!("Invalid transactionId"));
        }
        let AbiValue::Uint(_, transaction_id) = &transaction_id_abi_value.value else {
            return Err(anyhow!("Invalid transactionId"));
        };

        Ok(Self {
            custodian: Default::default(),
            transaction_id: transaction_id.to_u64().unwrap(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

impl MultisigSendTransaction {
    pub fn unpack(values: Vec<NamedAbiValue>) -> Result<Self> {
        if values.len() != 5 {
            return Err(anyhow!("Invalid number of arguments"));
        }

        let dest_abi_value = &values[0];
        let value_abi_value = &values[1];
        let bounce_abi_value = &values[2];
        let flags_abi_value = &values[3];
        let payload_abi_value = &values[4];

        if &*dest_abi_value.name != "dest" && dest_abi_value.value.get_type() != AbiType::Address {
            return Err(anyhow!("Invalid dest"));
        }
        let AbiValue::Address(dest) = &dest_abi_value.value else {
            return Err(anyhow!("Invalid dest"));
        };

        let AnyAddr::Std(dest) = *dest.clone() else {
            return Err(anyhow!("Invalid dest"));
        };

        if &*value_abi_value.name != "value"
            && value_abi_value.value.get_type() != AbiType::Uint(128)
        {
            return Err(anyhow!("Invalid value"));
        }
        let AbiValue::Uint(_, value) = &value_abi_value.value else {
            return Err(anyhow!("Invalid value"));
        };

        if &*bounce_abi_value.name != "bounce" && bounce_abi_value.value.get_type() != AbiType::Bool
        {
            return Err(anyhow!("Invalid bounce"));
        }
        let AbiValue::Bool(bounce) = &bounce_abi_value.value else {
            return Err(anyhow!("Invalid bounce"));
        };

        if &*flags_abi_value.name != "flags" && flags_abi_value.value.get_type() != AbiType::Uint(8)
        {
            return Err(anyhow!("Invalid flags"));
        }
        let AbiValue::Uint(_, flags) = &flags_abi_value.value else {
            return Err(anyhow!("Invalid flags"));
        };

        if &*payload_abi_value.name != "payload"
            && payload_abi_value.value.get_type() != AbiType::Cell
        {
            return Err(anyhow!("Invalid payload"));
        }
        let AbiValue::Cell(payload) = &payload_abi_value.value else {
            return Err(anyhow!("Invalid payload"));
        };

        Ok(Self {
            dest,
            value: value.to_u128().unwrap(),
            bounce: *bounce,
            flags: flags.to_u8().unwrap(),
            payload: payload.clone(),
        })
    }
}

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
        let output =
            MultisigConfirmTransaction::unpack(value.0).map_err(|_| UnpackerError::InvalidAbi)?;
        Ok(Self {
            custodian,
            transaction_id: output.transaction_id,
        })
    }
}

struct MultisigSubmitTransactionInput {
    dest: StdAddr,
    value: u128,
    bounce: bool,
    all_balance: bool,
    payload: Cell,
}

impl MultisigSubmitTransactionInput {
    pub fn unpack(values: Vec<NamedAbiValue>) -> Result<Self> {
        if values.len() != 5 {
            return Err(anyhow!("Invalid number of arguments"));
        }

        let dest_abi_value = &values[0];
        let value_abi_value = &values[1];
        let bounce_abi_value = &values[2];
        let all_balance_abi_value = &values[3];
        let payload_abi_value = &values[4];

        if &*dest_abi_value.name != "dest" && dest_abi_value.value.get_type() != AbiType::Address {
            return Err(anyhow!("Invalid dest"));
        }
        let AbiValue::Address(dest) = &dest_abi_value.value else {
            return Err(anyhow!("Invalid dest"));
        };

        let AnyAddr::Std(dest) = *dest.clone() else {
            return Err(anyhow!("Invalid dest"));
        };

        if &*value_abi_value.name != "value"
            && value_abi_value.value.get_type() != AbiType::Uint(128)
        {
            return Err(anyhow!("Invalid value"));
        }
        let AbiValue::Uint(_, value) = &value_abi_value.value else {
            return Err(anyhow!("Invalid value"));
        };

        if &*bounce_abi_value.name != "bounce" && bounce_abi_value.value.get_type() != AbiType::Bool
        {
            return Err(anyhow!("Invalid bounce"));
        }
        let AbiValue::Bool(bounce) = &bounce_abi_value.value else {
            return Err(anyhow!("Invalid bounce"));
        };

        if &*all_balance_abi_value.name != "allBalance"
            && all_balance_abi_value.value.get_type() != AbiType::Bool
        {
            return Err(anyhow!("Invalid allBalance"));
        }
        let AbiValue::Bool(all_balance) = &all_balance_abi_value.value else {
            return Err(anyhow!("Invalid allBalance"));
        };

        if &*payload_abi_value.name != "payload"
            && payload_abi_value.value.get_type() != AbiType::Cell
        {
            return Err(anyhow!("Invalid payload"));
        }
        let AbiValue::Cell(payload) = &payload_abi_value.value else {
            return Err(anyhow!("Invalid payload"));
        };

        Ok(Self {
            dest,
            value: value.to_u128().unwrap(),
            bounce: *bounce,
            all_balance: *all_balance,
            payload: payload.clone(),
        })
    }
}

struct MultisigSubmitTransactionOutput {
    trans_id: u64,
}

impl MultisigSubmitTransactionOutput {
    pub fn unpack(values: Vec<NamedAbiValue>) -> Result<Self> {
        if values.len() != 1 {
            return Err(anyhow!("Invalid number of arguments"));
        }

        let trans_id_abi_value = &values[0];
        if &*trans_id_abi_value.name != "transId"
            && trans_id_abi_value.value.get_type() != AbiType::Uint(64)
        {
            return Err(anyhow!("Invalid transId"));
        }
        let AbiValue::Uint(_, trans_id) = &trans_id_abi_value.value else {
            return Err(anyhow!("Invalid transId"));
        };

        Ok(Self {
            trans_id: trans_id.to_u64().unwrap(),
        })
    }
}

impl TryFrom<(HashBytes, ContractCall)> for MultisigSubmitTransaction {
    type Error = UnpackerError;

    fn try_from((custodian, value): (HashBytes, ContractCall)) -> Result<Self, Self::Error> {
        let input = MultisigSubmitTransactionInput::unpack(value.inputs)
            .map_err(|_| UnpackerError::InvalidAbi)?;
        let output = MultisigSubmitTransactionOutput::unpack(value.outputs)
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
        let input =
            MultisigSendTransaction::unpack(value.0).map_err(|_| UnpackerError::InvalidAbi)?;

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
        let input = multisig2::SubmitUpdateParams::unpack(value.inputs)
            .map_err(|_| UnpackerError::InvalidAbi)?;
        let output = multisig2::SubmitUpdateOutput::unpack(value.outputs)
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
        let input = multisig2::ConfirmUpdateParams::unpack(input.0)
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
        let input = multisig2::ExecuteUpdateParams::unpack(input.0)
            .map_err(|_| UnpackerError::InvalidAbi)?;
        Ok(Self {
            custodian,
            update_id: input.update_id,
        })
    }
}
