use anyhow::Result;
use bigdecimal::BigDecimal;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ton_core::*, utils::ton_wallet::MultisigType};

#[derive(thiserror::Error, Debug, Copy, Clone)]
pub enum TransactionError {
    #[error("Invalid transaction structure")]
    InvalidStructure,
    #[error("Unsupported transaction type")]
    Unsupported,
}

pub async fn parse_ton_transaction(
    account: HashBytes,
    block_utime: u32,
    transaction_hash: HashBytes,
    transaction: Transaction,
) -> Result<CaughtTonTransaction> {
    let in_msg = match transaction.in_msg.as_ref() {
        Some(in_msg_cell) => OwnedMessage::load_from(&mut in_msg_cell.as_slice()?)?,
        None => return Err(TransactionError::Unsupported.into()),
    };

    let address = StdAddr::new(0, account);

    let sender_address = get_sender_address(&transaction)?;
    let (sender_workchain_id, sender_hex) = match &sender_address {
        Some(address) => (
            Some(address.workchain as i32),
            Some(address.address.to_string()),
        ),
        None => (None, None),
    };

    let cell_builder = CellBuilder::build_from(&in_msg).map_err(anyhow::Error::from)?;
    let message_hash = cell_builder.repr_hash().to_string();
    let transaction_hash = Some(transaction_hash.to_string());
    let transaction_lt = BigDecimal::from_u64(transaction.lt);
    let transaction_scan_lt = Some(transaction.lt as i64);
    let transaction_timestamp = block_utime;
    let messages = Some(serde_json::to_value(get_messages(&transaction)?)?);
    let messages_hash = Some(serde_json::to_value(get_messages_hash(&transaction)?)?);
    let fee = BigDecimal::from_u128(compute_fees(&transaction));
    let value = BigDecimal::from_u128(compute_value(&transaction));
    let balance_change = BigDecimal::from_i128(compute_balance_change(&transaction));
    let multisig_transaction_id = nekoton::core::parsing::parse_multisig_transaction(
        MultisigType::SafeMultisigWallet,
        &transaction,
    )
    .and_then(|transaction| match transaction {
        MultisigTransaction::Confirm(transaction) => Some(transaction.transaction_id as i64),
        MultisigTransaction::Submit(transaction) => Some(transaction.trans_id as i64),
        _ => None,
    });

    let parsed = match in_msg.info {
        MsgInfo::Int(header) => {
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
                account_workchain_id: address.workchain as i32,
                account_hex: address.address.to_string(),
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
                multisig_transaction_id,
            })
        }
        MsgInfo::ExtIn(_) => {
            CaughtTonTransaction::UpdateSent(UpdateSentTransaction {
                message_hash,
                account_workchain_id: address.workchain as i32,
                account_hex: address.address.to_string(),
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
                    multisig_transaction_id,
                },
            })
        }
        MsgInfo::ExtOut(_) => return Err(TransactionError::InvalidStructure.into()),
    };

    Ok(parsed)
}

fn get_sender_address(transaction: &Transaction) -> Result<Option<StdAddr>> {
    let in_msg = transaction
        .load_in_msg()?
        .ok_or(TransactionError::InvalidStructure)?;
    match in_msg.info {
        MsgInfo::Int(info) => Ok(info.src.as_std().cloned()),
        MsgInfo::ExtIn(_) => Ok(None),
        MsgInfo::ExtOut(info) => Ok(info.src.as_std().cloned()),
    }
}

fn get_messages(transaction: &Transaction) -> Result<Vec<Message>> {
    let mut out_msgs = Vec::new();
    for message in transaction.iter_out_msgs() {
        let message = message?;
        let (fee, value, recipient) = match &message.info {
            MsgInfo::Int(info) => (
                Some(
                    BigDecimal::from_u128(info.fwd_fee.into_inner())
                        .ok_or(TransactionError::InvalidStructure)?,
                ),
                Some(
                    BigDecimal::from_u128(info.value.tokens.into_inner())
                        .ok_or(TransactionError::InvalidStructure)?,
                ),
                info.dst.as_std().map(|dst| MessageRecipient {
                    hex: dst.address.to_string(),
                    base64url: dst.display_base64_url(true).to_string(),
                    workchain_id: dst.workchain as i32,
                }),
            ),
            MsgInfo::ExtIn(info) => (
                None,
                None,
                info.dst.as_std().map(|dst| MessageRecipient {
                    hex: dst.address.to_string(),
                    base64url: dst.display_base64_url(true).to_string(),
                    workchain_id: dst.workchain as i32,
                }),
            ),
            MsgInfo::ExtOut(_) => (None, None, None),
        };

        let cell_builder = CellBuilder::build_from(&message)?;
        let message_hash = cell_builder.repr_hash().to_string();

        out_msgs.push(Message {
            fee,
            value,
            recipient,
            message_hash,
        });
    }

    Ok(out_msgs)
}

fn get_messages_hash(transaction: &Transaction) -> Result<Vec<String>> {
    let mut hashes = Vec::new();

    for message in transaction.iter_out_msgs() {
        let message = message?;
        let cell_builder = CellBuilder::build_from(&message)?;
        hashes.push(cell_builder.repr_hash().to_string());
    }

    Ok(hashes)
}

fn compute_value(transaction: &Transaction) -> u128 {
    let mut value = 0;

    if let Ok(Some(in_msg)) = transaction.load_in_msg() {
        if let MsgInfo::Int(header) = in_msg.info {
            value += header.value.tokens.into_inner();
        }
    }

    for message in transaction.iter_out_msgs() {
        let message = message.unwrap();
        if let MsgInfo::Int(header) = message.info {
            value += header.value.tokens.into_inner();
        }
    }

    value
}

fn compute_fees(transaction: &Transaction) -> u128 {
    let mut total_fees = 0;
    if let Ok(TxInfo::Ordinary(info)) = transaction.load_info() {
        total_fees += compute_total_transaction_fees(transaction, &info);
    }
    total_fees
}

pub fn compute_balance_change(transaction: &Transaction) -> i128 {
    let mut diff = 0;

    if let Ok(Some(in_msg)) = transaction.load_in_msg() {
        if let MsgInfo::Int(header) = in_msg.info {
            diff += header.value.tokens.into_inner() as i128;
        }
    }

    for message in transaction.iter_out_msgs() {
        let message = message.unwrap();
        if let MsgInfo::Int(header) = message.info {
            diff -= header.value.tokens.into_inner() as i128;
        }
    }

    if let Ok(TxInfo::Ordinary(info)) = transaction.load_info() {
        diff -= compute_total_transaction_fees(transaction, &info) as i128;
    }

    diff
}

pub fn compute_total_transaction_fees(transaction: &Transaction, info: &OrdinaryTxInfo) -> u128 {
    let mut total_fees = transaction.total_fees.tokens.into_inner();
    if let Some(phase) = &info.action_phase {
        total_fees += phase
            .total_fwd_fees
            .as_ref()
            .map(|tokens| tokens.into_inner())
            .unwrap_or_default();
        total_fees -= phase
            .total_action_fees
            .as_ref()
            .map(|tokens| tokens.into_inner())
            .unwrap_or_default();
    };
    if let Some(BouncePhase::Executed(phase)) = &info.bounce_phase {
        total_fees += phase.fwd_fees.into_inner();
    }
    total_fees
}

fn is_aborted(transaction: &Transaction) -> bool {
    let mut aborted = false;
    if let Ok(TxInfo::Ordinary(info)) = transaction.load_info() {
        aborted = info.aborted
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

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_transaction_with_message() -> Transaction {
        let binding = Boc::decode_base64(
            "te6ccgECEAEAAwgAA7d+QDCcWfS7Pd3OhqYgoQVempmo2OKQO5sOYx6EZBcyIbAAAuGThxKAhf1hAS\
            h02tBmYeWRHurQLFdhsiPgWeGNTbabaiPlZZ9gAALhk4cSgGZnCjIwADSAIfqQaAUEAQIXBAkFUFwjGIAhHJARA\
            wIAb8mKaBBMG8AMAAAAAAAEAAIAAAADVRiS8otLi359fajChkMh4j7YPNNVzsOUbNa9QsXWtVZBkDxsAJ5IegwV\
            xAgAAAAAAAAAAPsAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\
            AAAAAAAAAAAAAAIJy/7wuPdhy+h+DbYjvFbvGj3Bwqn3MOq5Y6hyrRTksTai0G7FEgWicR7eyflDhYqBuU4lk7Q\
            1nHs+PMBzAFRlACQIB4AwGAQHfBwGxSAHIBhOLPpdnu7nQ1MQUIKvTUzUbHFIHc2HMY9CMguZENwA4xAzS7nDkX\
            YkSKBG9Qn9FPKIp7rRePfQI63hB5LPI2tBQju+wBhvAOgAAXDJw4lASzOFGRsAIAWtw2J/JgAad35GR9lY9G036\
            2Q2Dq3ncMSd3A6aWdBC8YD5fdRqzgAAAAAAAAAAABHDeTfggABAJAUOAEFrRCE4/VsotLmBJtwSPkA/qwQuTKk2\
            yy6l1zSsQvy0wCgFDgB+vlzVCE32J+nPncYdENsw5VZeCw1GMZjfNdxFJz/7kEAsBQ4AQWtEITj9Wyi0uYEm3BI\
            +QD+rBC5MqTbLLqXXNKxC/LTAPAbFoAfr5c1QhN9ifpz53GHRDbMOVWXgsNRjGY3zXcRSc/+5BADkAwnFn0uz3d\
            zoamIKEFXpqZqNjikDubDmMehGQXMiG0FUFwjAGFEtcAABcMnDiUArM4UZGwA0Ba2eguV8AAAAAAAAAAAAjhvJv\
            wQAAgBBa0QhOP1bKLS5gSbcEj5AP6sELkypNssupdc0rEL8tMA4BQ4AQWtEITj9Wyi0uYEm3BI+QD+rBC5MqTbL\
            LqXXNKxC/LTgPAAA=",
        )
        .unwrap();
        let mut cell = binding.as_slice().unwrap();
        Transaction::load_from(&mut cell).unwrap()
    }

    fn mock_native_transaction() -> Transaction {
        let binding = Boc::decode_base64(
            "te6ccgECBQEAAQ8AA7VxLMcNYtT0Y0vHvF0Y6p6uYuZ3ru6E15MPbdMAiDOW+TAAAxF6kJyoOBX6Ew\
            /7kDzBL0X5vbiyJUQxs8oqMCx81lJVpHEGWGhQAALk17a3eDZv7sCQAABgJyfoAwIBABUMwE5PyQF9eEABIACCc\
            qeMvpds7qXtp0X7fcfK29e715cYDMD4djDoZFaoV2+IniF4UEqnl0mRBkkJUofiHH0OEnxt4bqWdhOvrktU02MB\
            AaAEALFIAQWtEITj9Wyi0uYEm3BI+QD+rBC5MqTbLLqXXNKxC/LTAASzHDWLU9GNLx7xdGOqermLmd67uhNeTD2\
            3TAIgzlvk0BfXhAAGCiwwAABiL1ITlQTN/dgSQA==",
        )
        .unwrap();
        let mut cell = binding.as_slice().unwrap();
        Transaction::load_from(&mut cell).unwrap()
    }

    fn mock_tip3_transaction() -> Transaction {
        let binding = Boc::decode_base64(
            "te6ccgECDAEAAl0AA7V/QK7VX0Cd/1ZlF9CjnQU/zjx5R/+gPcjjC/w75jghPeAAAxF68tckc9Q9f/\
            cDtEaGB89WcFPg7Kg/ufjqtloFybIORllBjolwAAMRevLXJGZv7tMQADR8gi0IBQQBAhUECQT+XD4YfDDMEQMCA\
            G/Jg9CQTAosIAAAAAAABAACAAAAA/Sl/SUL5ko0FMc/s2rL0MTaDiZjYIA0X+j0FcjV3p3wQFAWDACeRzeMFHQo\
            AAAAAAAAAADgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\
            AAAAAAAAAAACCcrofb9K77QB9Tu1i5S14jFHdFKo+C9REe8ROzOFr3AD7+kENKYFeCMxeVlP1W39Y9BCLCOLd3w\
            ZnvtGPEpk1yl8CAeAIBgEB3wcAsUgB6BXaq+gTv+rMovoUc6Cn+cePKP/0B7kcYX+HfMcEJ70AILWiEJx+rZRaX\
            MCTbgkfIB/VghcmVJtll1LrmlYhflpQR3xSoAYKLDAAAGIvXlrkkM392mJAAbFoAfr5c1QhN9ifpz53GHRDbMOV\
            WXgsNRjGY3zXcRSc/+5BAD0Cu1V9Anf9WZRfQo50FP848eUf/oD3I4wv8O+Y4IT3kE/lw+AGFEtcAABiL15a5Ir\
            N/dpiwAkBa2eguV8AAAAAAAAAAAAACRhOcqAAgBBa0QhOP1bKLS5gSbcEj5AP6sELkypNssupdc0rEL8tMAoBQ4\
            AQWtEITj9Wyi0uYEm3BI+QD+rBC5MqTbLLqXXNKxC/LSgLAAA=",
        )
        .unwrap();
        let mut cell = binding.as_slice().unwrap();
        Transaction::load_from(&mut cell).unwrap()
    }

    #[test]
    fn test_get_sender_address_with_message() {
        let transaction = mock_transaction_with_message();
        let result = get_sender_address(&transaction);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            Some(
                StdAddr::from_str(
                    "0:fd7cb9aa109bec4fd39f3b8c3a21b661caacbc161a8c6331be6bb88a4e7ff720"
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn test_get_sender_address_tip3() {
        let transaction = mock_tip3_transaction();
        let result = get_sender_address(&transaction);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            Some(
                StdAddr::from_str(
                    "0:fd7cb9aa109bec4fd39f3b8c3a21b661caacbc161a8c6331be6bb88a4e7ff720"
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn test_get_sender_address_native() {
        let transaction = mock_native_transaction();
        let result = get_sender_address(&transaction);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            Some(
                StdAddr::from_str(
                    "0:82d6884271fab6516973024db8247c807f56085c99526d965d4bae695885f969"
                )
                .unwrap()
            )
        );
    }
}
