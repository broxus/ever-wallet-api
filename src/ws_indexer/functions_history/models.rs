use nekoton::abi::num_traits;
use nekoton::abi::{uint256_bytes, UnpackAbi, UnpackToken, UnpackerError, UnpackerResult};

use ton_block::MsgAddressInt;
use ton_types::{Cell, UInt256};

#[derive(UnpackAbi, Debug, Clone)]
#[abi(plain)]
pub struct InternalTransfer {
    #[abi(uint128)]
    pub tokens: u128,
    #[abi(with = "uint256_bytes")]
    pub sender_public_key: UInt256,
    #[abi(address)]
    pub sender_address: MsgAddressInt,
    #[abi(address)]
    pub send_gas_to: MsgAddressInt,
    #[abi(bool)]
    pub notify_receiver: bool,
    #[abi(cell)]
    pub payload: Cell,
}

#[derive(UnpackAbi, Debug, Clone)]
#[abi(plain)]
pub struct InternalTransferBounced {
    #[abi(uint128)]
    pub tokens: u128,
}

#[derive(UnpackAbi, Debug, Clone)]
#[abi(plain)]
pub struct Accept {
    #[abi(uint128)]
    pub tokens: u128,
}

#[derive(UnpackAbi, Debug, Clone)]
#[abi(plain)]
pub struct TokensBurned {
    #[abi(uint128)]
    pub tokens: u128,
    #[abi(with = "uint256_bytes")]
    pub sender_public_key: UInt256,
    #[abi(address)]
    pub sender_address: MsgAddressInt,
    #[abi(address)]
    pub send_gas_to: MsgAddressInt,
    #[abi(address)]
    pub callback_address: MsgAddressInt,
    #[abi(cell)]
    pub callback_payload: Cell,
}

#[derive(UnpackAbi, Debug, Clone)]
#[abi(plain)]
pub struct TokensBurnedBounced {
    #[abi(uint128)]
    pub tokens: u128,
}
