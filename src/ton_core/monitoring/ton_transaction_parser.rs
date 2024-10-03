use anyhow::Result;
use bigdecimal::BigDecimal;
use nekoton::core::models::{MultisigTransaction, TransactionError};
use nekoton::core::ton_wallet::MultisigType;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use ton_block::CommonMsgInfo;
use ton_types::AccountId;
use uuid::Uuid;

use crate::ton_core::*;

pub async fn parse_ton_transaction(
    account: UInt256,
    block_utime: u32,
    transaction_hash: UInt256,
    transaction: ton_block::Transaction,
) -> Result<CaughtTonTransaction> {
    let in_msg = match &transaction.in_msg {
        Some(message) => message
            .read_struct()
            .map_err(|_| TransactionError::InvalidStructure)?,
        None => return Err(TransactionError::Unsupported.into()),
    };
    println!("{transaction:?}");
    let address = MsgAddressInt::with_standart(
        None,
        ton_block::BASE_WORKCHAIN_ID as i8,
        AccountId::from(account),
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
    let transaction_hash = Some(transaction_hash.to_hex_string());
    let transaction_lt = BigDecimal::from_u64(transaction.lt);
    let transaction_scan_lt = Some(transaction.lt as i64);
    let transaction_timestamp = block_utime;
    let messages = Some(serde_json::to_value(get_messages(&transaction)?)?);
    let messages_hash = Some(serde_json::to_value(get_messages_hash(&transaction)?)?);
    let fee = BigDecimal::from_u128(compute_fees(&transaction));
    let value = BigDecimal::from_u128(compute_value(&transaction));
    let balance_change = BigDecimal::from_i128(nekoton_utils::compute_balance_change(&transaction));
    let multisig_transaction_id = nekoton::core::parsing::parse_multisig_transaction(
        MultisigType::SafeMultisigWallet,
        &transaction,
    )
    .and_then(|transaction| match transaction {
        MultisigTransaction::Confirm(transaction) => Some(transaction.transaction_id as i64),
        MultisigTransaction::Submit(transaction) => Some(transaction.trans_id as i64),
        _ => None,
    });

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
                multisig_transaction_id,
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
                    multisig_transaction_id,
                },
            })
        }
        CommonMsgInfo::ExtOutMsgInfo(_) => return Err(TransactionError::InvalidStructure.into()),
    };

    Ok(parsed)
}

fn get_sender_address(transaction: &ton_block::Transaction) -> Result<Option<MsgAddressInt>> {
    let in_msg = transaction
        .in_msg
        .as_ref()
        .ok_or(TransactionError::InvalidStructure)?
        .read_struct()?;
    Ok(in_msg.src())
}

fn get_messages(transaction: &ton_block::Transaction) -> Result<Vec<Message>> {
    let mut out_msgs = Vec::new();
    transaction
        .out_msgs
        .iterate(|ton_block::InRefValue(item)| {
            let fee = match item.get_fee()? {
                Some(fee) => Some(
                    BigDecimal::from_u128(fee.as_u128())
                        .ok_or(TransactionError::InvalidStructure)?,
                ),
                None => None,
            };

            let value = match item.get_value() {
                Some(value) => Some(
                    BigDecimal::from_u128(value.grams.as_u128())
                        .ok_or(TransactionError::InvalidStructure)?,
                ),
                None => None,
            };

            let recipient = match item.header().get_dst_address() {
                Some(dst) => Some(MessageRecipient {
                    hex: dst.address().to_hex_string(),
                    base64url: nekoton_utils::pack_std_smc_addr(true, &dst, true)?,
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
            value += header.value.grams.as_u128();
        }
    }

    let _ = transaction.out_msgs.iterate(|out_msg| {
        if let CommonMsgInfo::IntMsgInfo(header) = out_msg.0.header() {
            value += header.value.grams.as_u128();
        }
        Ok(true)
    });

    value
}

fn compute_fees(transaction: &ton_block::Transaction) -> u128 {
    let mut fees = 0;
    if let Ok(ton_block::TransactionDescr::Ordinary(description)) =
        transaction.description.read_struct()
    {
        fees = nekoton_utils::compute_total_transaction_fees(transaction, &description)
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

#[cfg(test)]
mod tests {
    use super::*;
    use ton_block::{Deserializable, MsgAddressInt, Transaction};

    fn mock_transaction_with_message() -> Transaction {
        let transaction = Transaction::construct_from_base64(
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
        transaction
    }

    fn mock_transaction_without_message() -> Transaction {
        Transaction::default()
    }

    fn mock_native_transaction() -> Transaction {
        let transaction = Transaction::construct_from_base64(
            "te6ccgECBQEAAQ8AA7VxLMcNYtT0Y0vHvF0Y6p6uYuZ3ru6E15MPbdMAiDOW+TAAAxF6kJyoOBX6Ew\
            /7kDzBL0X5vbiyJUQxs8oqMCx81lJVpHEGWGhQAALk17a3eDZv7sCQAABgJyfoAwIBABUMwE5PyQF9eEABIACCc\
            qeMvpds7qXtp0X7fcfK29e715cYDMD4djDoZFaoV2+IniF4UEqnl0mRBkkJUofiHH0OEnxt4bqWdhOvrktU02MB\
            AaAEALFIAQWtEITj9Wyi0uYEm3BI+QD+rBC5MqTbLLqXXNKxC/LTAASzHDWLU9GNLx7xdGOqermLmd67uhNeTD2\
            3TAIgzlvk0BfXhAAGCiwwAABiL1ITlQTN/dgSQA==",
        )
            .unwrap();
        transaction
    }

    fn mock_tip3_transaction() -> Transaction {
        let transaction = Transaction::construct_from_base64(
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
        transaction
    }

    #[test]
    fn test_get_sender_address_with_message() {
        let transaction = mock_transaction_with_message();
        let result = get_sender_address(&transaction);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            Some(
                MsgAddressInt::from_str(
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
                MsgAddressInt::from_str(
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
                MsgAddressInt::from_str(
                    "0:82d6884271fab6516973024db8247c807f56085c99526d965d4bae695885f969"
                )
                    .unwrap()
            )
        );
    }


    #[test]
    fn test_get_sender_address_without_message() {
        // Simulate a transaction without an incoming message
        let transaction = mock_transaction_without_message();
        let result = get_sender_address(&transaction);
        assert!(result.is_err()); // Expect an error
    }
}
