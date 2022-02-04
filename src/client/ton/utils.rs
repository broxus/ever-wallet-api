use nekoton_abi::LastTransactionId;

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
