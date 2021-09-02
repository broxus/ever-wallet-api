use anyhow::Result;
use bigdecimal::BigDecimal;
use nekoton::core::models::TransactionError;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use ton_block::CommonMsgInfo;
use ton_types::AccountId;
use uuid::Uuid;

use crate::models::*;
use crate::ton_core::*;

pub async fn handle_transaction(transaction_ctx: TransactionContext) -> Result<ReceiveTransaction> {
    let transaction = transaction_ctx.transaction.clone();

    let in_msg = match &transaction.in_msg {
        Some(message) => message
            .read_struct()
            .map_err(|_| TransactionError::InvalidStructure)?,
        None => return Err(TransactionError::Unsupported.into()),
    };

    let (account_workchain_id, account_hex) = {
        let address = MsgAddressInt::with_standart(
            None,
            ton_block::BASE_WORKCHAIN_ID as i8,
            AccountId::from(transaction_ctx.account),
        )?;
        (address.workchain_id(), address.address().to_hex_string())
    };

    let message_hash = in_msg.hash()?.to_hex_string();
    let transaction_hash = Some(transaction_ctx.transaction_hash.to_hex_string());
    let transaction_lt = BigDecimal::from_u64(transaction.lt);
    let transaction_scan_lt = Some(transaction_ctx.transaction.lt as i64);
    let (sender_workchain_id, sender_hex) = get_sender_info(&transaction)?;
    let messages = Some(serde_json::to_value(get_messages(&transaction)?)?);
    let fee = BigDecimal::from_u64(compute_fees(&transaction));
    let value = BigDecimal::from_u128(compute_value(&transaction));
    let balance_change =
        BigDecimal::from_i64(nekoton::core::utils::compute_balance_change(&transaction));

    // check if sender is token wallet (in cache) and check if owner of this token wallet this recipient

    let parsed = match in_msg.header() {
        CommonMsgInfo::IntMsgInfo(header) => {
            ReceiveTransaction::Create(CreateReceiveTransaction {
                id: Uuid::new_v4(),
                message_hash,
                transaction_hash,
                transaction_lt,
                transaction_timeout: None,
                transaction_scan_lt,
                sender_workchain_id,
                sender_hex,
                account_workchain_id,
                account_hex,
                messages,
                data: None,             // TODO
                original_value: None,   // TODO
                original_outputs: None, // TODO
                value,
                fee,
                balance_change,
                direction: TonTransactionDirection::Receive,
                status: TonTransactionStatus::Done,
                error: None,
                aborted: is_aborted(&transaction),
                bounce: header.bounce,
                sender_is_token_wallet: false, //TODO
            })
        }
        CommonMsgInfo::ExtInMsgInfo(_) => {
            ReceiveTransaction::UpdateSent(UpdateSentTransaction {
                message_hash,
                account_workchain_id,
                account_hex,
                input: UpdateSendTransaction {
                    transaction_hash,
                    transaction_lt,
                    transaction_timeout: None,
                    transaction_scan_lt,
                    sender_workchain_id,
                    sender_hex,
                    messages,
                    data: None,             // TODO
                    original_value: None,   // TODO
                    original_outputs: None, // TODO
                    value,
                    fee,
                    balance_change,
                    status: TonTransactionStatus::Done,
                    error: None,
                },
            })
        }
        CommonMsgInfo::ExtOutMsgInfo(_) => return Err(TransactionError::InvalidStructure.into()),
    };

    Ok(parsed)
}

fn get_sender_info(transaction: &ton_block::Transaction) -> Result<(Option<i32>, Option<String>)> {
    let in_msg = match &transaction.in_msg {
        Some(message) => message
            .read_struct()
            .map(nekoton::core::models::Message::from)
            .map_err(|_| TransactionError::InvalidStructure)?,
        None => return Err(TransactionError::Unsupported.into()),
    };

    Ok(match in_msg.src {
        Some(address) => (
            Some(address.workchain_id()),
            Some(address.address().to_hex_string()),
        ),
        None => (None, None),
    })
}

fn get_messages(transaction: &ton_block::Transaction) -> Result<Vec<Message>> {
    let mut out_msgs = Vec::new();
    transaction
        .out_msgs
        .iterate(|ton_block::InRefValue(item)| {
            let dst = item
                .header()
                .get_dst_address()
                .ok_or(TransactionError::InvalidStructure)?;

            let fee =
                BigDecimal::from_u128(item.get_fee()?.ok_or(TransactionError::InvalidStructure)?.0)
                    .ok_or(TransactionError::InvalidStructure)?;

            let value = BigDecimal::from_u128(
                item.get_value()
                    .ok_or(TransactionError::InvalidStructure)?
                    .grams
                    .0,
            )
            .ok_or(TransactionError::InvalidStructure)?;

            out_msgs.push(Message {
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
        })
        .map_err(|_| TransactionError::InvalidStructure)?;

    Ok(out_msgs)
}

fn compute_value(transaction: &ton_block::Transaction) -> u128 {
    let mut value = 0;

    if let Some(in_msg) = transaction
        .in_msg
        .as_ref()
        .and_then(|data| data.read_struct().ok())
    {
        if let ton_block::CommonMsgInfo::IntMsgInfo(header) = in_msg.header() {
            value += header.value.grams.0;
        }
    }

    let _ = transaction.out_msgs.iterate(|out_msg| {
        if let CommonMsgInfo::IntMsgInfo(header) = out_msg.0.header() {
            value += header.value.grams.0;
        }
        Ok(true)
    });

    value
}

fn compute_fees(transaction: &ton_block::Transaction) -> u64 {
    let mut fees = 0;
    if let Ok(ton_block::TransactionDescr::Ordinary(description)) =
        transaction.description.read_struct()
    {
        fees = nekoton::core::utils::compute_total_transaction_fees(transaction, &description)
    }
    fees
}

fn is_aborted(transaction: &ton_block::Transaction) -> bool {
    let mut aborted = false;
    if let Ok(ton_block::TransactionDescr::Ordinary(description)) =
        transaction.description.read_struct()
    {
        aborted = description.aborted
    }
    aborted
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
