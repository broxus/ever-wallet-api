use nekoton_abi::LastTransactionId;
use tycho_types::models::OwnedMessage;

use crate::models::SentTransaction;

pub fn parse_last_transaction(
    last_transaction: &LastTransactionId,
) -> (Option<String>, Option<String>) {
    let (last_transaction_hash, last_transaction_lt) = match last_transaction {
        LastTransactionId::Exact(transaction_id) => (
            Some(transaction_id.hash.to_hex_string()),
            Some(transaction_id.lt.to_string()),
        ),
        LastTransactionId::Inexact { .. } => (None, None),
    };

    (last_transaction_hash, last_transaction_lt)
}

#[derive(Debug)]
pub struct PrepareResult {
    pub sent_transaction: SentTransaction,
    pub owned_message: OwnedMessage,
    pub expired_at: u32,
}
