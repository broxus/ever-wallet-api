use std::convert::TryFrom;

use anyhow::Result;
use tycho_types::{
    abi::Function,
    cell::{Cell, CellSlice, HashBytes, Load},
    models::{MsgInfo, OwnedMessage, StdAddr, Transaction},
};

use crate::utils::{
    multisig::models::{
        MultisigConfirmTransaction, MultisigConfirmUpdate, MultisigExecuteUpdate,
        MultisigSendTransaction, MultisigSubmitTransaction, MultisigSubmitUpdate,
        MultisigTransaction,
    },
    ton_wallet::{multisig::UnpackerError, MultisigType},
    wallets::{
        multisig::{self},
        multisig2,
    },
    ContractCall, InputMessage,
};

pub fn parse_multisig_transaction(
    multisig_type: MultisigType,
    tx: &Transaction,
) -> Option<MultisigTransaction> {
    let in_msg_cell = tx.in_msg.as_ref()?;

    let Ok(mut slice) = in_msg_cell.as_slice() else {
        return None;
    };
    let Ok(in_msg) = OwnedMessage::load_from(&mut slice) else {
        return None;
    };

    if !matches!(in_msg.info, MsgInfo::ExtIn(_)) {
        return None;
    }
    parse_multisig_transaction_impl(multisig_type, in_msg, tx)
}

fn parse_multisig_transaction_impl(
    multisig_type: MultisigType,
    in_msg: OwnedMessage,
    tx: &Transaction,
) -> Option<MultisigTransaction> {
    const PUBKEY_OFFSET: usize = 1 + ed25519_dalek::SIGNATURE_LENGTH * 8 + 1;
    const PUBKEY_LENGTH: usize = 256;
    const TIME_LENGTH: usize = 64;
    const EXPIRE_LENGTH: usize = 32;

    let body = in_msg.body;
    let Ok(mut body_slice) = body.clone().1.as_slice() else {
        return None;
    };

    // Shift body by Maybe(signature), Maybe(pubkey), time and expire
    body_slice
        .skip_first(
            (PUBKEY_OFFSET + PUBKEY_LENGTH + TIME_LENGTH + EXPIRE_LENGTH) as u16,
            0,
        )
        .ok()?;

    let Ok(function_id) = body_slice.get_u32(0) else {
        return None;
    };

    let parse_tx_input =
        |function: &Function, mut slice: CellSlice<'_>| -> Option<(HashBytes, InputMessage)> {
            let inputs = function.decode_internal_input(slice).ok()?;
            slice.skip_first(PUBKEY_OFFSET as u16, 0).ok()?;
            let custodian = slice.load_u256().ok()?;
            Some((custodian, InputMessage(inputs)))
        };

    let parse_tx_full = |function: &Function, body: Cell| -> Option<(HashBytes, ContractCall)> {
        let Ok(mut slice) = body.as_slice() else {
            return None;
        };
        let (custodian, InputMessage(inputs)) = parse_tx_input(function, slice)?;
        let outputs = function.parse(tx).ok()?;
        Some((custodian, ContractCall { inputs, outputs }))
    };

    let functions = MultisigFunctions::instance(multisig_type);

    if function_id == functions.send_transaction.input_id {
        let inputs = functions
            .send_transaction
            .decode_input(body, false, false)
            .ok()?;
        MultisigSendTransaction::try_from(InputMessage(inputs))
            .map(MultisigTransaction::Send)
            .ok()
    } else if function_id == functions.submit_transaction.input_id {
        let (custodian, value) = parse_tx_full(functions.submit_transaction, body)?;
        MultisigSubmitTransaction::try_from((custodian, value))
            .map(MultisigTransaction::Submit)
            .ok()
    } else if function_id == functions.confirm_transaction.input_id {
        let (custodian, value) = parse_tx_input(functions.confirm_transaction, body)?;
        MultisigConfirmTransaction::try_from((custodian, value))
            .map(MultisigTransaction::Confirm)
            .ok()
    } else if let Some(functions) = &functions.update_functions {
        if function_id == functions.submit_update.input_id {
            let (custodian, value) = parse_tx_full(functions.submit_update, body)?;
            MultisigSubmitUpdate::try_from((custodian, value))
                .map(MultisigTransaction::SubmitUpdate)
                .ok()
        } else if function_id == functions.confirm_update.input_id {
            let (custodian, value) = parse_tx_input(functions.confirm_update, body)?;
            MultisigConfirmUpdate::try_from((custodian, value))
                .map(MultisigTransaction::ConfirmUpdate)
                .ok()
        } else if function_id == functions.execute_update.input_id {
            let (custodian, value) = parse_tx_input(functions.execute_update, body)?;
            MultisigExecuteUpdate::try_from((custodian, value))
                .map(MultisigTransaction::ExecuteUpdate)
                .ok()
        } else {
            None
        }
    } else {
        None
    }
}

struct MultisigFunctions {
    send_transaction: &'static Function,
    submit_transaction: &'static Function,
    confirm_transaction: &'static Function,
    update_functions: Option<UpdateFunctions>,
}

struct UpdateFunctions {
    submit_update: &'static Function,
    confirm_update: &'static Function,
    execute_update: &'static Function,
}

impl MultisigFunctions {
    fn instance(multisig_type: MultisigType) -> &'static Self {
        static OLD_FUNCTIONS: std::sync::OnceLock<MultisigFunctions> = std::sync::OnceLock::new();
        static NEW_FUNCTIONS: std::sync::OnceLock<MultisigFunctions> = std::sync::OnceLock::new();

        match multisig_type {
            ty if ty.is_multisig2() => NEW_FUNCTIONS.get_or_init(|| MultisigFunctions {
                send_transaction: multisig2::send_transaction(),
                submit_transaction: multisig2::submit_transaction(),
                confirm_transaction: multisig2::confirm_transaction(),
                update_functions: Some(UpdateFunctions {
                    submit_update: multisig2::submit_update(),
                    confirm_update: multisig2::confirm_update(),
                    execute_update: multisig2::execute_update(),
                }),
            }),
            _ => OLD_FUNCTIONS.get_or_init(|| MultisigFunctions {
                send_transaction: multisig::send_transaction(),
                submit_transaction: multisig::submit_transaction(),
                confirm_transaction: multisig::confirm_transaction(),
                update_functions: None,
            }),
        }
    }
}

impl TryFrom<(HashBytes, InputMessage)> for MultisigConfirmTransaction {
    type Error = UnpackerError;

    fn try_from((custodian, value): (HashBytes, InputMessage)) -> Result<Self, Self::Error> {
        let output: MultisigConfirmTransaction = value.0.unpack()?;
        Ok(Self {
            custodian,
            transaction_id: output.transaction_id,
        })
    }
}

#[derive(UnpackAbiPlain)]
struct MultisigSubmitTransactionInput {
    #[abi(address)]
    dest: StdAddr,
    #[abi(with = "uint128_number")]
    value: u128,
    #[abi(bool)]
    bounce: bool,
    #[abi(bool, name = "allBalance")]
    all_balance: bool,
    #[abi(cell)]
    payload: Cell,
}

#[derive(UnpackAbiPlain)]
struct MultisigSubmitTransactionOutput {
    #[abi(uint64, name = "transId")]
    trans_id: u64,
}

impl TryFrom<(HashBytes, ContractCall)> for MultisigSubmitTransaction {
    type Error = UnpackerError;

    fn try_from((custodian, value): (HashBytes, ContractCall)) -> Result<Self, Self::Error> {
        let input: MultisigSubmitTransactionInput = value.inputs.unpack()?;
        let output: MultisigSubmitTransactionOutput = value.outputs.unpack()?;

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
        let input: MultisigSendTransaction = value.0.unpack()?;

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
        let input: multisig2::SubmitUpdateParams = value.inputs.unpack()?;
        let output: multisig2::SubmitUpdateOutput = value.outputs.unpack()?;

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
        use nekoton_contracts::wallets::multisig2;

        let input: multisig2::ConfirmUpdateParams = input.0.unpack()?;
        Ok(Self {
            custodian,
            update_id: input.update_id,
        })
    }
}

impl TryFrom<(HashBytes, InputMessage)> for MultisigExecuteUpdate {
    type Error = UnpackerError;

    fn try_from((custodian, input): (HashBytes, InputMessage)) -> Result<Self, Self::Error> {
        use nekoton_contracts::wallets::multisig2;

        let input: multisig2::ExecuteUpdateParams = input.0.unpack()?;
        Ok(Self {
            custodian,
            update_id: input.update_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn parse_transaction(data: &str) -> (Transaction, TransactionDescrOrdinary) {
        let tx = Transaction::construct_from_base64(data).unwrap();
        let description = match tx.description.read_struct().unwrap() {
            TransactionDescr::Ordinary(description) => description,
            _ => panic!(),
        };
        (tx, description)
    }

    #[test]
    fn test_parse_multisig_submit() {
        let tx = Transaction::construct_from_base64("te6ccgECDAEAAkMAA693d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3AAAEv38uN8H+CfBrFklcU0i9Vs4RZzxi5vtTa9PqJ/LpPctz/rat2wAABIjJ0UsBX2sytAADQIBQQBAgcMBgRAAwIAYcAAAAAAAAIAAAAAAAOylU78GhKKYOUuj1Rh3dLpOOzgJUEyoySchhaM60lDREBQDowAnUfXAxOIAAAAAAAAAABtAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAAgnJ5QDnTzA46E1KOsPz7QLrshaiw53aaaTNY7TZfFM9uf9wCstMqmz8MmfSmYLSpRuMah9ruqiOVsRPjzhTEdu9aAgHgCAYBAd8HAHXn+7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u4AAAJfv5cb4S+1mVoSY7BZq+1mVo/lxvgwAFFif7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7gwJAeGUlZeW3g4p7fOroeyZUZdj1hWrKWusR/Na6V9uRhKJvV3dgWDQ1/YR5hQfYLaM861DgLJMku/LPDKMt43TyJUH+ToLdTA3yCwRnsc9IMg9JIXlsbI92/1mZ+RrZF1GGY1AAABdLq+AHhfazLsEx2CzYAoBY4AVKmRhQN1a9YbnwdGmdH0KtPv2SINcG4FpEDjh70ON2qAAAAAAAAAAAAAdjv+NHoAUCwAA").unwrap();

        let custodian =
            HashBytes::from_str("e4e82dd4c0df20b0467b1cf48320f4921796c6c8f76ff5999f91ad9175186635")
                .unwrap();

        assert!(matches!(
            parse_transaction_additional_info(
                &tx,
                WalletType::Multisig(MultisigType::SafeMultisigWallet)
            )
            .unwrap(),
            TransactionAdditionalInfo::WalletInteraction(WalletInteractionInfo {
                recipient: Some(_),
                known_payload: None,
                method: WalletInteractionMethod::Multisig(data)
            }) if matches!(&*data, MultisigTransaction::Submit(submit) if submit.custodian == custodian)
        ));
    }

    #[test]
    fn test_parse_multisig_confirm() {
        let tx = Transaction::construct_from_base64("te6ccgECCgEAAjAAA693d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3AAAJcbrc/8GSsRcwsaEKUmFwdbT9tmaf3vKqKpeWIR9/9GyMA8r2+gAACXGutDTBYBvSYwADQIBQQBAgcMBgRAAwIAYcAAAAAAAAIAAAAAAAI1K3sqU+I63UTJ+xkdHcyrkM2hxcBJu//z7hF+/hEtukBQFcwAnUYtYxOIAAAAAAAAAABSwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAAgnITMJnhiVklA89yLWhQU+4BB1tJ3iPLRRZoWlPVKSkbvYENWnQphG03/JbEJJWwJbdhZCl+oH7UI7ARqCUcU6H/AgHgCAYBAd8HAK9J/u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7vACa4ZyAEEjOHCY7aEkcDRTMruTfdNxrg9GyWxKU18Pes2WvMQekAAAAAABLjdbn/hMA3pMZAAUWJ/u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7uDAkA8c+cpxQ8FYd2C/XWiibmIX4wPfvHIultapCNOhW5dJ5hl2YD+PHO24RUXdbY669yR8BUfGNuxVTwVkV1K0HA7QByTARuQhGj9eozhRteIImtsExhdcFckfL9FqBq5uNuaoAAAF3bK3Ps2Ab0p4ap0DtYBvF9mf0BgGA=").unwrap();

        let custodian =
            HashBytes::from_str("c93011b908468fd7a8ce146d788226b6c13185d7057247cbf45a81ab9b8db9aa")
                .unwrap();

        assert!(matches!(
            parse_transaction_additional_info(
                &tx,
                WalletType::Multisig(MultisigType::SafeMultisigWallet)
            )
            .unwrap(),
            TransactionAdditionalInfo::WalletInteraction(WalletInteractionInfo {
                recipient: None,
                known_payload: None,
                method: WalletInteractionMethod::Multisig(data)
            }) if matches!(&*data, MultisigTransaction::Confirm(confirm) if confirm.custodian == custodian)
        ))
    }
}
