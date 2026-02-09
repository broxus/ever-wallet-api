use std::sync::Arc;

use anyhow::{anyhow, Result};
use num_traits::ToPrimitive;
use tycho_types::{
    abi::{
        AbiType, AbiValue, FromAbi, Function, IntoAbi, NamedAbiType, NamedAbiValue, WithAbiType,
    },
    cell::{Cell, HashBytes},
    models::StdAddr,
};

use crate::utils::declare_function;

pub fn constructor() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "constructor",
        inputs: vec![
            AbiType::Array(Arc::new(AbiType::Uint(256))).named("owners"),
            AbiType::Uint(8).named("reqConfirms"),
            AbiType::Uint(32).named("lifetime"),
        ],
        outputs: vec![] as Vec<NamedAbiType>,
    }
}

pub fn send_transaction() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "sendTransaction",
        inputs: vec![
            AbiType::Address.named("dest"),
            AbiType::Uint(128).named("value"),
            AbiType::Bool.named("bounce"),
            AbiType::Uint(8).named("flags"),
            AbiType::Cell.named("payload"),
        ],
        outputs: vec![] as Vec<NamedAbiType>,
    }
}

pub fn submit_transaction() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "submitTransaction",
        inputs: vec![
            AbiType::Address.named("dest"),
            AbiType::Uint(128).named("value"),
            AbiType::Bool.named("bounce"),
            AbiType::Bool.named("allBalance"),
            AbiType::Cell.named("payload"),
            AbiType::Optional(Arc::new(AbiType::Cell)).named("stateInit"),
        ],
        outputs: vec![
            AbiType::Uint(64).named("transId")
        ],
    }
}

pub fn confirm_transaction() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "confirmTransaction",
        inputs: vec![
            AbiType::Uint(64).named("transactionId"),
            ],
        outputs: vec![] as Vec<NamedAbiType>,
    }
}

#[derive(Debug)]
pub struct MultisigTransaction {
    pub id: u64,
    pub confirmation_mask: u32,
    pub signs_required: u8,
    pub signs_received: u8,
    pub creator: HashBytes,
    pub index: u8,
    pub dest: StdAddr,
    pub value: u128,
    pub send_flags: u16,
    pub payload: Cell,
    pub bounce: bool,
    pub state_init: Option<Cell>,
}

impl WithAbiType for MultisigTransaction {
    fn abi_type() -> AbiType {
        AbiType::Tuple(Arc::new([
            AbiType::Uint(64).named("id"),
            AbiType::Uint(32).named("confirmationMask"),
            AbiType::Uint(8).named("signsRequired"),
            AbiType::Uint(8).named("signsReceived"),
            AbiType::Uint(256).named("creator"),
            AbiType::Uint(8).named("index"),
            AbiType::Address.named("dest"),
            AbiType::Uint(128).named("value"),
            AbiType::Uint(16).named("sendFlags"),
            AbiType::Cell.named("payload"),
            AbiType::Bool.named("bounce"),
            AbiType::Optional(Arc::new(AbiType::Cell)).named("stateInit"),
        ]))
    }
}

pub fn get_transactions() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "getTransactions",
        inputs: vec![] as Vec<NamedAbiType>,
        outputs: vec![
            AbiType::Array(Arc::new(MultisigTransaction::abi_type())).named("transactions")
        ]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MultisigCustodian {
    pub index: u8,
    pub pubkey: HashBytes,
}

impl WithAbiType for MultisigCustodian {
    fn abi_type() -> AbiType {
        AbiType::Tuple(Arc::new([
            AbiType::Uint(8).named("index"),
            AbiType::Uint(256).named("pubkey"),
        ]))
    }
}

pub fn get_custodians() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "getCustodians",
        inputs: vec![] as Vec<NamedAbiType>,
        outputs: vec![
            AbiType::Array(Arc::new(MultisigCustodian::abi_type())).named("custodians")
        ]
    }
}

#[derive(Debug, Clone)]
pub struct SubmitUpdateParams {
    pub code_hash: Option<HashBytes>,
    pub owners: Option<Vec<HashBytes>>,
    pub req_confirms: Option<u8>,
    pub lifetime: Option<u64>,
}

impl SubmitUpdateParams {
    fn abi_type() -> Vec<NamedAbiType> {
        vec![
            AbiType::Optional(Arc::new(AbiType::Uint(256))).named("codeHash"),
            AbiType::Optional(Arc::new(AbiType::Array(Arc::new(AbiType::Uint(256)))))
                .named("owners"),
            AbiType::Optional(Arc::new(AbiType::Uint(8))).named("reqConfirms"),
            AbiType::Optional(Arc::new(AbiType::Uint(64))).named("lifetime"),
        ]
    }

    pub fn abi_values(&self) -> Vec<NamedAbiValue> {
        vec![
            self.code_hash.as_abi().named("codeHash"),
            self.owners.as_abi().named("owners"),
            self.req_confirms.as_abi().named("reqConfirms"),
            self.lifetime.as_abi().named("lifetime"),
        ]
    }

    pub fn unpack(values: Vec<NamedAbiValue>) -> Result<Self> {
        if values.len() != 4 {
            return Err(anyhow!("Invalid number of arguments"));
        }

        let code_hash_abi_value = &values[0];
        let owners_abi_value = &values[1];
        let req_confirms_abi_value = &values[2];
        let lifetime_abi_value = &values[3];

        if &*code_hash_abi_value.name != "codeHash"
            && code_hash_abi_value.value.get_type()
                != AbiType::Optional(Arc::new(AbiType::Uint(256)))
        {
            return Err(anyhow!("Invalid code_hash"));
        }
        let AbiValue::Optional(_, code_hash) = &code_hash_abi_value.value else {
            return Err(anyhow!("Invalid code_hash"));
        };

        let code_hash = if let Some(code_hash) = code_hash {
            let AbiValue::Uint(_, code_hash) = &**code_hash else {
                return Err(anyhow!("Invalid code_hash"));
            };
            Some(HashBytes::from_slice(&code_hash.to_bytes_be()))
        } else {
            None
        };

        if &*owners_abi_value.name != "owners"
            && owners_abi_value.value.get_type()
                != AbiType::Optional(Arc::new(AbiType::Array(Arc::new(AbiType::Uint(256)))))
        {
            return Err(anyhow!("Invalid owners"));
        }
        let AbiValue::Optional(_, owners) = &owners_abi_value.value else {
            return Err(anyhow!("Invalid owners"));
        };

        let owners = if let Some(owners) = owners {
            let AbiValue::Array(_, owners) = &**owners else {
                return Err(anyhow!("Invalid owners"));
            };

            let mut owners_res: Vec<HashBytes> = Vec::with_capacity(owners.len());
            for owner in owners {
                let AbiValue::Uint(_, owner) = owner else {
                    return Err(anyhow!("Invalid owner"));
                };
                owners_res.push(HashBytes::from_slice(&owner.to_bytes_be()));
            }
            Some(owners_res)
        } else {
            None
        };

        if &*req_confirms_abi_value.name != "reqConfirms"
            && req_confirms_abi_value.value.get_type()
                != AbiType::Optional(Arc::new(AbiType::Uint(8)))
        {
            return Err(anyhow!("Invalid reqConfirms"));
        }
        let AbiValue::Optional(_, req_confirms) = &req_confirms_abi_value.value else {
            return Err(anyhow!("Invalid reqConfirms"));
        };

        let req_confirms = if let Some(req_confirms) = req_confirms {
            let AbiValue::Uint(_, req_confirms) = &**req_confirms else {
                return Err(anyhow!("Invalid reqConfirms"));
            };
            (*req_confirms).to_u8()
        } else {
            None
        };

        if &*lifetime_abi_value.name != "lifetime"
            && lifetime_abi_value.value.get_type() != AbiType::Optional(Arc::new(AbiType::Uint(64)))
        {
            return Err(anyhow!("Invalid lifetime"));
        }
        let AbiValue::Optional(_, lifetime) = &lifetime_abi_value.value else {
            return Err(anyhow!("Invalid lifetime"));
        };

        let lifetime = if let Some(lifetime) = lifetime {
            let AbiValue::Uint(_, lifetime) = &**lifetime else {
                return Err(anyhow!("Invalid lifetime"));
            };
            (*lifetime).to_u64()
        } else {
            None
        };

        Ok(SubmitUpdateParams {
            code_hash,
            owners,
            req_confirms,
            lifetime,
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SubmitUpdateOutput {
    pub update_id: u64,
}

impl SubmitUpdateOutput {
    fn abi_type() -> Vec<NamedAbiType> {
        vec![AbiType::Uint(64).named("updateId")]
    }

    pub fn unpack(values: Vec<NamedAbiValue>) -> Result<Self> {
        if values.len() != 1 {
            return Err(anyhow!("Invalid number of arguments"));
        }

        let update_id_abi_value = &values[0];
        if &*update_id_abi_value.name != "updateId"
            && update_id_abi_value.value.get_type() != AbiType::Uint(64)
        {
            return Err(anyhow!("Invalid updateId"));
        }
        let AbiValue::Uint(_, update_id) = &update_id_abi_value.value else {
            return Err(anyhow!("Invalid updateId"));
        };

        Ok(SubmitUpdateOutput {
            update_id: update_id.to_u64().unwrap(),
        })
    }
}

pub fn submit_update() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "submitUpdate",
        inputs: SubmitUpdateParams::abi_type(),
        outputs: SubmitUpdateOutput::abi_type(),
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ConfirmUpdateParams {
    pub update_id: u64,
}

impl ConfirmUpdateParams {
    fn abi_type() -> Vec<NamedAbiType> {
        vec![AbiType::Uint(64).named("updateId")]
    }

    pub fn abi_values(&self) -> Vec<NamedAbiValue> {
        vec![self.update_id.as_abi().named("updateId")]
    }
    pub fn unpack(values: Vec<NamedAbiValue>) -> Result<Self> {
        if values.len() != 1 {
            return Err(anyhow!("Invalid number of arguments"));
        }

        let update_id_abi_value = &values[0];
        if &*update_id_abi_value.name != "updateId"
            && update_id_abi_value.value.get_type() != AbiType::Uint(64)
        {
            return Err(anyhow!("Invalid updateId"));
        }
        let AbiValue::Uint(_, update_id) = &update_id_abi_value.value else {
            return Err(anyhow!("Invalid updateId"));
        };

        Ok(ConfirmUpdateParams {
            update_id: update_id.to_u64().unwrap(),
        })
    }
}

pub fn confirm_update() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "confirmUpdate",
        inputs: ConfirmUpdateParams::abi_type(),
        outputs: vec![] as Vec<NamedAbiType>,
    }
}

#[derive(Debug, Clone)]
pub struct ExecuteUpdateParams {
    pub update_id: u64,
    pub code: Option<Cell>,
}

impl ExecuteUpdateParams {
    fn abi_type() -> Vec<NamedAbiType> {
        vec![
            AbiType::Uint(64).named("updateId"),
            AbiType::Optional(Arc::new(AbiType::Cell)).named("code"),
        ]
    }

    pub fn abi_values(&self) -> Vec<NamedAbiValue> {
        vec![
            self.update_id.as_abi().named("updateId"),
            self.code.as_abi().named("code"),
        ]
    }

    pub fn unpack(values: Vec<NamedAbiValue>) -> Result<Self> {
        if values.len() != 2 {
            return Err(anyhow!("Invalid number of arguments"));
        }

        let update_id_abi_value = &values[0];
        let code_abi_value = &values[1];

        if &*update_id_abi_value.name != "updateId"
            && update_id_abi_value.value.get_type() != AbiType::Uint(64)
        {
            return Err(anyhow!("Invalid updateId"));
        }
        let AbiValue::Uint(_, update_id) = &update_id_abi_value.value else {
            return Err(anyhow!("Invalid updateId"));
        };

        if &*code_abi_value.name != "code"
            && code_abi_value.value.get_type() != AbiType::Optional(Arc::new(AbiType::Cell))
        {
            return Err(anyhow!("Invalid code"));
        }
        let AbiValue::Optional(_, code) = &code_abi_value.value else {
            return Err(anyhow!("Invalid code"));
        };

        let code = if let Some(code) = code {
            let AbiValue::Cell(code) = &**code else {
                return Err(anyhow!("Invalid code"));
            };
            Some(code.clone())
        } else {
            None
        };

        Ok(ExecuteUpdateParams {
            update_id: update_id.to_u64().unwrap(),
            code,
        })
    }
}

pub fn execute_update() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "executeUpdate",
        inputs: ExecuteUpdateParams::abi_type(),
        outputs: vec![] as Vec<NamedAbiType>,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SetCodeMultisigParams {
    pub max_queued_transactions: u8,
    pub max_custodian_count: u8,
    pub expiration_time: u64,
    pub min_value: u128,
    pub required_txn_confirms: u8,
    pub required_upd_confirms: u8,
}

impl SetCodeMultisigParams {
    fn abi_type() -> Vec<NamedAbiType> {
        vec![
            AbiType::Uint(8).named("maxQueuedTransactions"),
            AbiType::Uint(8).named("maxCustodianCount"),
            AbiType::Uint(64).named("expirationTime"),
            AbiType::Uint(128).named("minValue"),
            AbiType::Uint(8).named("requiredTxnConfirms"),
            AbiType::Uint(8).named("requiredUpdConfirms"),
        ]
    }
}
pub fn get_parameters() -> &'static Function {
    declare_function! {
        abi: v2_3,
        header: [pubkey, time, expire],
        name: "getParameters",
        inputs: vec![] as Vec<NamedAbiType>,
        outputs: SetCodeMultisigParams::abi_type(),
    }
}

#[derive(IntoAbi, FromAbi, WithAbiType, Eq, PartialEq, Debug, Clone)]
pub struct UpdateTransaction {
    pub id: u64,
    pub index: u8,
    pub signs: u8,
    #[abi(name = "confirmationsMask")]
    pub confirmations_mask: u32,
    pub creator: HashBytes,
    #[abi(name = "newCodeHash")]
    pub new_code_hash: Option<HashBytes>,
    #[abi(name = "newCustodians")]
    pub new_custodians: Option<Vec<HashBytes>>,
    #[abi(name = "newReqConfirms")]
    pub new_req_confirms: Option<u8>,
    #[abi(name = "newLifetime")]
    pub new_lifetime: Option<u32>,
}

pub mod v2_0 {
    use tycho_types::abi::NamedAbiType;

    use super::*;

    pub fn get_update_requests() -> &'static Function {
        declare_function! {
            abi: v2_3,
            header: [pubkey, time, expire],
            name: "getUpdateRequests",
            inputs: vec![] as Vec<NamedAbiType>,
            outputs: {
                let param_types = match UpdateTransaction::abi_type() {
                    AbiType::Tuple(params) => {
                        let mut vec = params.iter().cloned().collect::<Vec<_>>();
                        if let Some(last) = vec.last_mut() {
                            if let NamedAbiType { ty: AbiType::Optional(param), name } = last {
                                if let AbiType::Uint(_) = param.as_ref() {
                                    *last = AbiType::Optional(Arc::new(AbiType::Uint(64))).named(&*name.clone());
                                }
                            }
                        }
                        AbiType::Tuple(Arc::<[NamedAbiType]>::from(vec))
                    }
                    other => other,
                };

                vec![
                    AbiType::Array(Arc::new(param_types)).named("updates")
                ]
            },
        }
    }
}

pub mod v2_1 {
    use super::*;

    pub fn get_update_requests() -> &'static Function {
        declare_function! {
            abi: v2_3,
            header: [pubkey, time, expire],
            name: "getUpdateRequests",
            inputs: vec![] as Vec<NamedAbiType>,
            outputs: vec![
                AbiType::Array(Arc::new(UpdateTransaction::abi_type())).named("updates")
            ],
        }
    }
}
