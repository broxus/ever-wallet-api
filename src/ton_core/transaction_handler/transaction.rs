use anyhow::Result;
use bigdecimal::BigDecimal;
use nekoton::core::models::TransactionError;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use ton_block::CommonMsgInfo;
use ton_types::AccountId;
use uuid::Uuid;

use crate::models::account_enums::{TonTransactionDirection, TonTransactionStatus};
use crate::ton_core::*;

pub async fn handle_transaction(transaction_ctx: TransactionContext) -> Result<ReceiveTransaction> {
    let data = transaction_ctx.transaction.clone();

    let desc = if let ton_block::TransactionDescr::Ordinary(desc) =
        data.description
            .read_struct()
            .map_err(|_| TransactionError::InvalidStructure)?
    {
        desc
    } else {
        return Err(TransactionError::Unsupported.into());
    };

    let in_msg = match &data.in_msg {
        Some(message) => message
            .read_struct()
            .map(nekoton::core::models::Message::from)
            .map_err(|_| TransactionError::InvalidStructure)?,
        None => return Err(TransactionError::Unsupported.into()),
    };

    let raw_in_msg = match &data.in_msg {
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

    let (sender_workchain_id, sender_hex) = match in_msg.src {
        Some(address) => (
            Some(address.workchain_id()),
            Some(address.address().to_hex_string()),
        ),
        None => (None, None),
    };

    let messages = Some(serde_json::to_value(get_messages(&data)?)?);

    let fee = BigDecimal::from_u64(nekoton::core::utils::compute_total_transaction_fees(
        &data, &desc,
    ));

    let balance_change = BigDecimal::from_i64(nekoton::core::utils::compute_balance_change(&data));

    let value = BigDecimal::from_u64(in_msg.value);
    let message_hash = raw_in_msg.hash()?.to_hex_string();
    let transaction_lt = BigDecimal::from_u64(data.lt);
    let transaction_scan_lt = Some(transaction_ctx.transaction.lt as i64);
    let transaction_hash = Some(transaction_ctx.transaction_hash.to_hex_string());

    let parsed = match raw_in_msg.header() {
        CommonMsgInfo::IntMsgInfo(_) => {
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
                status: TonTransactionStatus::New,
                error: None,
                aborted: desc.aborted,
                bounce: in_msg.bounce,
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

fn get_messages(data: &ton_block::Transaction) -> Result<Vec<Message>> {
    let mut out_msgs = Vec::new();
    data.out_msgs
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
