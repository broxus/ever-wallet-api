use std::convert::TryFrom;

use tycho_types::{
    abi::Function,
    cell::{CellSlice, HashBytes, Load},
    models::{MsgInfo, OwnedMessage, Transaction},
};

use crate::utils::{
    multisig::models::{
        MultisigConfirmTransaction, MultisigConfirmUpdate, MultisigExecuteUpdate,
        MultisigSendTransaction, MultisigSubmitTransaction, MultisigSubmitUpdate,
        MultisigTransaction,
    },
    ton_wallet::MultisigType,
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
    let Ok(mut body_slice) = body.1.as_slice() else {
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

    let parse_tx_full =
        |function: &Function, body_slice: CellSlice| -> Option<(HashBytes, ContractCall)> {
            let (custodian, InputMessage(inputs)) = parse_tx_input(function, body_slice)?;
            let mut output = None;
            for out_msg in tx.iter_out_msgs() {
                let Ok(out_msg) = out_msg else {
                    continue;
                };

                if !matches!(out_msg.info, MsgInfo::ExtOut(_)) {
                    continue;
                }

                let body = out_msg.body;

                let Ok(function_id) = body.get_u32(0) else {
                    continue;
                };

                if function.output_id == function_id {
                    let Ok(tokens) = function.decode_output(body) else {
                        continue;
                    };

                    output = Some(tokens);
                    break;
                }
            }

            Some((
                custodian,
                ContractCall {
                    inputs,
                    outputs: output.unwrap_or_default(),
                },
            ))
        };

    let functions = MultisigFunctions::instance(multisig_type);

    if function_id == functions.send_transaction.input_id {
        let inputs = functions
            .send_transaction
            .decode_external_input(body_slice)
            .ok()?;
        MultisigSendTransaction::try_from(InputMessage(inputs))
            .map(MultisigTransaction::Send)
            .ok()
    } else if function_id == functions.submit_transaction.input_id {
        let (custodian, value) = parse_tx_full(functions.submit_transaction, body_slice)?;
        MultisigSubmitTransaction::try_from((custodian, value))
            .map(MultisigTransaction::Submit)
            .ok()
    } else if function_id == functions.confirm_transaction.input_id {
        let (custodian, value) = parse_tx_input(functions.confirm_transaction, body_slice)?;
        MultisigConfirmTransaction::try_from((custodian, value))
            .map(MultisigTransaction::Confirm)
            .ok()
    } else if let Some(functions) = &functions.update_functions {
        if function_id == functions.submit_update.input_id {
            let (custodian, value) = parse_tx_full(functions.submit_update, body_slice)?;
            MultisigSubmitUpdate::try_from((custodian, value))
                .map(MultisigTransaction::SubmitUpdate)
                .ok()
        } else if function_id == functions.confirm_update.input_id {
            let (custodian, value) = parse_tx_input(functions.confirm_update, body_slice)?;
            MultisigConfirmUpdate::try_from((custodian, value))
                .map(MultisigTransaction::ConfirmUpdate)
                .ok()
        } else if function_id == functions.execute_update.input_id {
            let (custodian, value) = parse_tx_input(functions.execute_update, body_slice)?;
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

#[cfg(test)]
mod tests {

    // use tycho_types::{
    //     boc::Boc,
    //     cell::Load,
    //     models::{OrdinaryTxInfo, TxInfo},
    // };

    // use super::*;

    // fn parse_transaction(data: &str) -> (Transaction, OrdinaryTxInfo) {
    //     let binding = Boc::decode_base64(data).unwrap();
    //     let mut cell = binding.as_slice().unwrap();
    //     let transaction = Transaction::load_from(&mut cell).unwrap();
    //     let info = match transaction.load_info().unwrap() {
    //         TxInfo::Ordinary(info) => info,
    //         _ => panic!(),
    //     };
    //     (transaction, info)
    // }

    //#[test]
    //fn test_parse_multisig_submit() {
    //    let tx = Transaction::construct_from_base64("te6ccgECDAEAAkMAA693d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3AAAEv38uN8H+CfBrFklcU0i9Vs4RZzxi5vtTa9PqJ/LpPctz/rat2wAABIjJ0UsBX2sytAADQIBQQBAgcMBgRAAwIAYcAAAAAAAAIAAAAAAAOylU78GhKKYOUuj1Rh3dLpOOzgJUEyoySchhaM60lDREBQDowAnUfXAxOIAAAAAAAAAABtAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAAgnJ5QDnTzA46E1KOsPz7QLrshaiw53aaaTNY7TZfFM9uf9wCstMqmz8MmfSmYLSpRuMah9ruqiOVsRPjzhTEdu9aAgHgCAYBAd8HAHXn+7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u4AAAJfv5cb4S+1mVoSY7BZq+1mVo/lxvgwAFFif7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7gwJAeGUlZeW3g4p7fOroeyZUZdj1hWrKWusR/Na6V9uRhKJvV3dgWDQ1/YR5hQfYLaM861DgLJMku/LPDKMt43TyJUH+ToLdTA3yCwRnsc9IMg9JIXlsbI92/1mZ+RrZF1GGY1AAABdLq+AHhfazLsEx2CzYAoBY4AVKmRhQN1a9YbnwdGmdH0KtPv2SINcG4FpEDjh70ON2qAAAAAAAAAAAAAdjv+NHoAUCwAA").unwrap();
    //
    //    let custodian =
    //        HashBytes::from_str("e4e82dd4c0df20b0467b1cf48320f4921796c6c8f76ff5999f91ad9175186635")
    //            .unwrap();
    //
    //    assert!(matches!(
    //        parse_transaction_additional_info(
    //            &tx,
    //            WalletType::Multisig(MultisigType::SafeMultisigWallet)
    //        )
    //        .unwrap(),
    //        TransactionAdditionalInfo::WalletInteraction(WalletInteractionInfo {
    //            recipient: Some(_),
    //            known_payload: None,
    //            method: WalletInteractionMethod::Multisig(data)
    //        }) if matches!(&*data, MultisigTransaction::Submit(submit) if submit.custodian == custodian)
    //    ));
    //}
    //
    //#[test]
    //fn test_parse_multisig_confirm() {
    //    let tx = Transaction::construct_from_base64("te6ccgECCgEAAjAAA693d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3AAAJcbrc/8GSsRcwsaEKUmFwdbT9tmaf3vKqKpeWIR9/9GyMA8r2+gAACXGutDTBYBvSYwADQIBQQBAgcMBgRAAwIAYcAAAAAAAAIAAAAAAAI1K3sqU+I63UTJ+xkdHcyrkM2hxcBJu//z7hF+/hEtukBQFcwAnUYtYxOIAAAAAAAAAABSwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAAgnITMJnhiVklA89yLWhQU+4BB1tJ3iPLRRZoWlPVKSkbvYENWnQphG03/JbEJJWwJbdhZCl+oH7UI7ARqCUcU6H/AgHgCAYBAd8HAK9J/u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7vACa4ZyAEEjOHCY7aEkcDRTMruTfdNxrg9GyWxKU18Pes2WvMQekAAAAAABLjdbn/hMA3pMZAAUWJ/u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7u7uDAkA8c+cpxQ8FYd2C/XWiibmIX4wPfvHIultapCNOhW5dJ5hl2YD+PHO24RUXdbY669yR8BUfGNuxVTwVkV1K0HA7QByTARuQhGj9eozhRteIImtsExhdcFckfL9FqBq5uNuaoAAAF3bK3Ps2Ab0p4ap0DtYBvF9mf0BgGA=").unwrap();
    //
    //    let custodian =
    //        HashBytes::from_str("c93011b908468fd7a8ce146d788226b6c13185d7057247cbf45a81ab9b8db9aa")
    //            .unwrap();
    //
    //    assert!(matches!(
    //        parse_transaction_additional_info(
    //            &tx,
    //            WalletType::Multisig(MultisigType::SafeMultisigWallet)
    //        )
    //        .unwrap(),
    //        TransactionAdditionalInfo::WalletInteraction(WalletInteractionInfo {
    //            recipient: None,
    //            known_payload: None,
    //            method: WalletInteractionMethod::Multisig(data)
    //        }) if matches!(&*data, MultisigTransaction::Confirm(confirm) if confirm.custodian == custodian)
    //    ))
    //}
}
