use std::str::FromStr;

use bigdecimal::BigDecimal;
use nekoton_utils::pack_std_smc_addr;
use serde::{Deserialize, Serialize};
use ton_block::MsgAddressInt;
use uuid::Uuid;

use crate::models::*;

#[derive(Debug, Serialize, Deserialize, Clone, derive_more::Constructor, opg::OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("AccountTokenTransactionEventResponse")]
pub struct AccountTransactionEvent {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub message_hash: String,
    pub account: AddressResponse,
    #[opg("balanceChange", string, optional)]
    pub balance_change: Option<BigDecimal>,
    pub root_address: Option<String>,
    pub transaction_direction: TonTransactionDirection,
    pub transaction_status: TonTransactionStatus,
    pub event_status: TonEventStatus,
    pub sender_is_token_wallet: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<TokenTransactionEventDb> for AccountTransactionEvent {
    fn from(t: TokenTransactionEventDb) -> Self {
        let account =
            MsgAddressInt::from_str(&format!("{}:{}", t.account_workchain_id, t.account_hex))
                .unwrap();
        let base64url = Address(pack_std_smc_addr(true, &account, false).unwrap());

        Self {
            id: t.id,
            transaction_id: t.token_transaction_id,
            message_hash: t.message_hash,
            account: AddressResponse {
                workchain_id: t.account_workchain_id,
                hex: Address(t.account_hex),
                base64url,
            },
            balance_change: Some(t.value),
            root_address: Some(t.root_address),
            transaction_direction: t.transaction_direction,
            transaction_status: t.transaction_status.into(),
            event_status: t.event_status,
            sender_is_token_wallet: false,
            created_at: t.created_at.timestamp_millis(),
            updated_at: t.updated_at.timestamp_millis(),
        }
    }
}

impl From<TransactionEventDb> for AccountTransactionEvent {
    fn from(t: TransactionEventDb) -> Self {
        let account =
            MsgAddressInt::from_str(&format!("{}:{}", t.account_workchain_id, t.account_hex))
                .unwrap();
        let base64url = Address(pack_std_smc_addr(true, &account, false).unwrap());

        Self {
            id: t.id,
            transaction_id: t.transaction_id,
            message_hash: t.message_hash,
            account: AddressResponse {
                workchain_id: t.account_workchain_id,
                hex: Address(t.account_hex),
                base64url,
            },
            balance_change: t.balance_change,
            root_address: None,
            transaction_direction: t.transaction_direction,
            transaction_status: t.transaction_status,
            event_status: t.event_status,
            sender_is_token_wallet: t.sender_is_token_wallet,
            created_at: t.created_at.timestamp_millis(),
            updated_at: t.updated_at.timestamp_millis(),
        }
    }
}
