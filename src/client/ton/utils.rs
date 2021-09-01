use nekoton_abi::LastTransactionId;
use ton_block::AccountState;

use crate::models::*;

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

pub fn transform_account_state(account_state: &AccountState) -> AccountStatus {
    match account_state {
        AccountState::AccountUninit => AccountStatus::UnInit,
        AccountState::AccountActive(_) => AccountStatus::Active,
        AccountState::AccountFrozen(_) => AccountStatus::Frozen,
    }
}
