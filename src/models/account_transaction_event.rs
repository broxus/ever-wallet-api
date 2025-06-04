use std::str::FromStr;

use bigdecimal::BigDecimal;
use everscale_types::models::StdAddr;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::*;

#[derive(Debug, Serialize, Deserialize, Clone, derive_more::Constructor, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountTokenTransactionEventResponse")]
pub struct AccountTransactionEvent {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub transaction_hash: Option<String>,
    pub message_hash: String,
    pub owner_message_hash: Option<String>,
    pub account: Account,
    pub sender: Option<Account>,
    #[opg("balanceChange", string, optional)]
    pub balance_change: Option<BigDecimal>,
    pub root_address: Option<String>,
    pub transaction_direction: TonTransactionDirection,
    pub transaction_status: TonTransactionStatus,
    pub event_status: TonEventStatus,
    pub multisig_transaction_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<TokenTransactionEventDb> for AccountTransactionEvent {
    fn from(t: TokenTransactionEventDb) -> Self {
        let account =
            StdAddr::from_str(&format!("{}:{}", t.account_workchain_id, t.account_hex)).unwrap();
        let base64url = Address(account.display_base64_url(true).to_string());

        let sender = if let (Some(sender_workchain_id), Some(sender_hex)) =
            (t.sender_workchain_id, t.sender_hex)
        {
            let sender =
                StdAddr::from_str(&format!("{}:{}", sender_workchain_id, sender_hex)).unwrap();
            let base64url = Address(sender.display_base64_url(true).to_string());
            Some(Account {
                workchain_id: sender_workchain_id,
                hex: Address(sender_hex),
                base64url,
            })
        } else {
            None
        };

        Self {
            id: t.id,
            transaction_id: t.token_transaction_id,
            transaction_hash: t.token_transaction_hash,
            message_hash: t.message_hash,
            owner_message_hash: t.owner_message_hash,
            account: Account {
                workchain_id: t.account_workchain_id,
                hex: Address(t.account_hex),
                base64url,
            },
            sender,
            balance_change: Some(t.value),
            root_address: Some(t.root_address),
            transaction_direction: t.transaction_direction,
            transaction_status: t.transaction_status.into(),
            event_status: t.event_status,
            multisig_transaction_id: None,
            created_at: t.created_at.and_utc().timestamp_millis(),
            updated_at: t.updated_at.and_utc().timestamp_millis(),
        }
    }
}

impl From<TransactionEventDb> for AccountTransactionEvent {
    fn from(t: TransactionEventDb) -> Self {
        let account =
            StdAddr::from_str(&format!("{}:{}", t.account_workchain_id, t.account_hex)).unwrap();
        let base64url = Address(account.display_base64_url(true).to_string());

        let sender = if let (Some(sender_workchain_id), Some(sender_hex)) =
            (t.sender_workchain_id, t.sender_hex)
        {
            let sender =
                StdAddr::from_str(&format!("{}:{}", sender_workchain_id, sender_hex)).unwrap();
            let base64url = Address(sender.display_base64_url(true).to_string());
            Some(Account {
                workchain_id: sender_workchain_id,
                hex: Address(sender_hex),
                base64url,
            })
        } else {
            None
        };

        Self {
            id: t.id,
            transaction_id: t.transaction_id,
            transaction_hash: t.transaction_hash,
            message_hash: t.message_hash,
            owner_message_hash: None,
            account: Account {
                workchain_id: t.account_workchain_id,
                hex: Address(t.account_hex),
                base64url,
            },
            sender,
            balance_change: t.balance_change,
            root_address: None,
            transaction_direction: t.transaction_direction,
            transaction_status: t.transaction_status,
            event_status: t.event_status,
            multisig_transaction_id: t.multisig_transaction_id,
            created_at: t.created_at.and_utc().timestamp_millis(),
            updated_at: t.updated_at.and_utc().timestamp_millis(),
        }
    }
}
