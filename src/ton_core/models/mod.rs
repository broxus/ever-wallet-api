use num_bigint::BigUint;
use ton_block::{AccountState, MsgAddressInt};
use ton_types::UInt256;

#[derive(Debug)]
pub struct TonAddressInfo {
    pub workchain_id: i32,
    pub hex: String,
    pub network_balance: u128,
    pub account_status: AccountState,
    pub last_transaction_lt: Option<u64>,
    pub last_transaction_hash: Option<UInt256>,
}

#[derive(Debug)]
pub struct TokenAddressInfo {
    pub workchain_id: i32,
    pub hex: String,
    pub root_address: MsgAddressInt,
    pub network_balance: BigUint,
    pub account_status: AccountState,
    pub last_transaction_lt: Option<u64>,
    pub last_transaction_hash: Option<UInt256>,
}
