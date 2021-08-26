use std::str::FromStr;

use anyhow::Result;
use bigdecimal::BigDecimal;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use ton_block::{CommonMsgInfo, MsgAddressIntOrNone, TransactionDescr};
use uuid::Uuid;

use crate::models::account_enums::{TonTransactionDirection, TonTransactionStatus};
use crate::ton_core::*;

pub async fn handle_transaction(
    transaction_ctx: TransactionContext,
) -> Result<Option<ReceiveTransaction>> {
    let mut parsed = None;

    if let Some(in_msg) = transaction_ctx
        .transaction
        .in_msg
        .as_ref()
        .and_then(|data| data.read_struct().ok())
    {
        let account_address = MsgAddressInt::from_str(&transaction_ctx.account.to_hex_string())?;

        let mut messages = Vec::new();
        let mut transaction_value: u128 = Default::default();
        transaction_ctx
            .transaction
            .out_msgs
            .iterate(|ton_block::InRefValue(item)| {
                let dst = item
                    .header()
                    .get_dst_address()
                    .ok_or(TonCoreError::WrongTransaction)?;

                let fee = item.get_fee()?.ok_or(TonCoreError::WrongTransaction)?.0;
                let fee = BigDecimal::from_u128(fee).ok_or(TonCoreError::WrongTransaction)?;

                let value = item
                    .get_value()
                    .ok_or(TonCoreError::WrongTransaction)?
                    .grams
                    .0;
                transaction_value += value;
                let value = BigDecimal::from_u128(value).ok_or(TonCoreError::WrongTransaction)?;

                messages.push(Message {
                    fee,
                    value,
                    recipient: MessageRecipient {
                        hex: dst.address().to_hex_string(),
                        base64url: nekoton_utils::pack_std_smc_addr(true, &dst, false)?,
                        workchain_id: dst.workchain_id(),
                    },
                    message_hash: item.hash()?.to_hex_string(),
                });

                Ok(true)
            })?;

        let transaction_sescription = match transaction_ctx.transaction.description.read_struct()? {
            TransactionDescr::Ordinary(description) => description,
            _ => return Err(TonCoreError::WrongTransaction.into()),
        };
        let fee = BigDecimal::from_u64(nekoton::core::utils::compute_total_transaction_fees(
            &transaction_ctx.transaction,
            &transaction_sescription,
        ));

        parsed = match in_msg.header() {
            CommonMsgInfo::IntMsgInfo(message_header) => {
                let (sender_workchain_id, sender_hex) = match &message_header.src {
                    MsgAddressIntOrNone::Some(addr) => (
                        Some(addr.get_workchain_id()),
                        Some(addr.address().to_hex_string()),
                    ),
                    MsgAddressIntOrNone::None => (None, None),
                };

                Some(ReceiveTransaction::Create(CreateReceiveTransaction {
                    id: Uuid::new_v4(),
                    message_hash: in_msg.hash()?.to_hex_string(),
                    transaction_hash: Some(transaction_ctx.transaction_hash.to_hex_string()),
                    transaction_lt: BigDecimal::from_u64(transaction_ctx.transaction.lt),
                    transaction_timeout: None,
                    transaction_scan_lt: Some(transaction_ctx.transaction.lt as i64),
                    sender_workchain_id,
                    sender_hex,
                    account_workchain_id: account_address.workchain_id(),
                    account_hex: account_address.address().to_hex_string(),
                    messages: None,
                    data: None,
                    original_value: None,
                    original_outputs: None,
                    value: BigDecimal::from_u128(transaction_value),
                    fee,
                    balance_change: None, // TODO: -(value) - fee
                    direction: TonTransactionDirection::Receive,
                    status: TonTransactionStatus::New,
                    error: None,
                    aborted: false,
                    bounce: false,
                }))
            }
            CommonMsgInfo::ExtInMsgInfo(_) => {
                Some(ReceiveTransaction::UpdateSent(UpdateSentTransaction {
                    message_hash: in_msg.hash()?.to_hex_string(),
                    account_workchain_id: account_address.workchain_id(),
                    account_hex: account_address.address().to_hex_string(),
                    input: UpdateSendTransaction {
                        transaction_hash: Some(transaction_ctx.transaction_hash.to_hex_string()),
                        transaction_lt: BigDecimal::from_u64(transaction_ctx.transaction.lt),
                        transaction_timeout: None,
                        transaction_scan_lt: Some(transaction_ctx.transaction.lt as i64),
                        sender_workchain_id: None,
                        sender_hex: None,
                        messages: Some(serde_json::to_value(messages)?),
                        data: None,           // TODO
                        original_value: None, // TODO
                        original_outputs: None,
                        value: BigDecimal::from_u128(transaction_value),
                        fee,
                        balance_change: None, // TODO: value - fee (for out -value-fee)
                        status: TonTransactionStatus::Done,
                        error: None,
                    },
                }))
            }
            CommonMsgInfo::ExtOutMsgInfo(_) => return Err(TonCoreError::WrongTransaction.into()),
        };
    }

    Ok(parsed)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub fee: BigDecimal,
    pub value: BigDecimal,
    pub recipient: MessageRecipient,
    pub message_hash: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageRecipient {
    pub hex: String,
    pub base64url: String,
    pub workchain_id: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Outputs {
    pub value: BigDecimal,
    pub recipient: OutputsRecipient,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputsRecipient {
    pub hex: String,
    pub base64url: String,
    pub workchain_id: i64,
}
