use num_bigint::BigInt;
use ton_block::AccountState;
use ton_types::UInt256;

#[derive(Debug)]
pub struct TonAddressInfo {
    pub workchain_id: i32,
    pub hex: String,
    pub network_balance: BigInt,
    pub account_status: AccountState,
    pub last_transaction_lt: Option<u64>,
    pub last_transaction_hash: Option<UInt256>,
    pub sync_u_time: i64,
}

#[derive(Debug)]
pub struct TokenAddressInfo {
    pub workchain_id: i32,
    pub hex: String,
    pub root_address: UInt256,
    pub network_balance: BigInt,
    pub account_status: AccountState,
    pub last_transaction_lt: Option<u64>,
    pub last_transaction_hash: Option<UInt256>,
    pub sync_u_time: i64,
}
