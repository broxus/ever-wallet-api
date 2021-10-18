use anyhow::Result;
use bigdecimal::BigDecimal;
use nekoton::core::models::TransactionError;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use ton_block::CommonMsgInfo;
use ton_types::AccountId;
use uuid::Uuid;

use crate::ton_core::*;

pub async fn parse_ton_transaction(event: TonTransactionEvent) -> Result<CaughtTonTransaction> {
    log::info!("Parse ton transaction");

    let transaction = event.transaction.clone();

    let in_msg = match &transaction.in_msg {
        Some(message) => message
            .read_struct()
            .map_err(|_| TransactionError::InvalidStructure)?,
        None => return Err(TransactionError::Unsupported.into()),
    };

    let address = MsgAddressInt::with_standart(
        None,
        ton_block::BASE_WORKCHAIN_ID as i8,
        AccountId::from(event.account),
    )?;

    let sender_address = get_sender_address(&transaction)?;
    let (sender_workchain_id, sender_hex) = match &sender_address {
        Some(address) => (
            Some(address.workchain_id()),
            Some(address.address().to_hex_string()),
        ),
        None => (None, None),
    };

    let message_hash = in_msg.hash()?.to_hex_string();
    let transaction_hash = Some(event.transaction_hash.to_hex_string());
    let transaction_lt = BigDecimal::from_u64(transaction.lt);
    let transaction_scan_lt = Some(event.transaction.lt as i64);
    let transaction_timestamp = event.block_utime;
    let messages = Some(serde_json::to_value(get_messages(&transaction)?)?);
    let messages_hash = Some(serde_json::to_value(get_messages_hash(&transaction)?)?);
    let fee = BigDecimal::from_u64(compute_fees(&transaction));
    let value = BigDecimal::from_u128(compute_value(&transaction));
    let balance_change =
        BigDecimal::from_i64(nekoton::core::utils::compute_balance_change(&transaction));

    let parsed = match in_msg.header() {
        CommonMsgInfo::IntMsgInfo(header) => {
            CaughtTonTransaction::Create(CreateReceiveTransaction {
                id: Uuid::new_v4(),
                message_hash,
                transaction_hash,
                transaction_lt,
                transaction_timeout: None,
                transaction_scan_lt,
                transaction_timestamp,
                sender_workchain_id,
                sender_hex,
                account_workchain_id: address.workchain_id(),
                account_hex: address.address().to_hex_string(),
                messages,
                messages_hash,
                data: None, // TODO
                original_value: None,
                original_outputs: None,
                value,
                fee,
                balance_change,
                direction: TonTransactionDirection::Receive,
                status: TonTransactionStatus::Done,
                error: None,
                aborted: is_aborted(&transaction),
                bounce: header.bounce,
            })
        }
        CommonMsgInfo::ExtInMsgInfo(_) => {
            CaughtTonTransaction::UpdateSent(UpdateSentTransaction {
                message_hash,
                account_workchain_id: address.workchain_id(),
                account_hex: address.address().to_hex_string(),
                input: UpdateSendTransaction {
                    transaction_hash,
                    transaction_lt,
                    transaction_scan_lt,
                    transaction_timestamp: Some(transaction_timestamp),
                    sender_workchain_id,
                    sender_hex,
                    messages,
                    messages_hash,
                    data: None, // TODO
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

fn get_sender_address(transaction: &ton_block::Transaction) -> Result<Option<MsgAddressInt>> {
    let in_msg = match &transaction.in_msg {
        Some(message) => message
            .read_struct()
            .map(nekoton::core::models::Message::from)
            .map_err(|_| TransactionError::InvalidStructure)?,
        None => return Err(TransactionError::Unsupported.into()),
    };

    Ok(in_msg.src)
}

fn get_messages(transaction: &ton_block::Transaction) -> Result<Vec<Message>> {
    let mut out_msgs = Vec::new();
    transaction
        .out_msgs
        .iterate(|ton_block::InRefValue(item)| {
            let fee = match item.get_fee()? {
                Some(fee) => {
                    Some(BigDecimal::from_u128(fee.0).ok_or(TransactionError::InvalidStructure)?)
                }
                None => None,
            };

            let value = match item.get_value() {
                Some(value) => Some(
                    BigDecimal::from_u128(value.grams.0)
                        .ok_or(TransactionError::InvalidStructure)?,
                ),
                None => None,
            };

            let recipient = match item.header().get_dst_address() {
                Some(dst) => Some(MessageRecipient {
                    hex: dst.address().to_hex_string(),
                    base64url: nekoton_utils::pack_std_smc_addr(true, &dst, false)?,
                    workchain_id: dst.workchain_id(),
                }),
                None => None,
            };

            out_msgs.push(Message {
                fee,
                value,
                recipient,
                message_hash: item.hash()?.to_hex_string(),
            });

            Ok(true)
        })
        .map_err(|_| TransactionError::InvalidStructure)?;

    Ok(out_msgs)
}

fn get_messages_hash(transaction: &ton_block::Transaction) -> Result<Vec<String>> {
    let mut hashes = Vec::new();
    transaction
        .out_msgs
        .iterate(|ton_block::InRefValue(item)| {
            hashes.push(item.hash()?.to_hex_string());

            Ok(true)
        })
        .map_err(|_| TransactionError::InvalidStructure)?;

    Ok(hashes)
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
struct Message {
    pub fee: Option<BigDecimal>,
    pub value: Option<BigDecimal>,
    pub recipient: Option<MessageRecipient>,
    pub message_hash: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessageRecipient {
    pub hex: String,
    pub base64url: String,
    pub workchain_id: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Outputs {
    pub value: BigDecimal,
    pub recipient: OutputsRecipient,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OutputsRecipient {
    pub hex: String,
    pub base64url: String,
    pub workchain_id: i64,
}
