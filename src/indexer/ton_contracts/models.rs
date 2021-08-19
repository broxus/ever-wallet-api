use ton_types::UInt256;

use nekoton_abi::*;

#[derive(Debug, Clone, PackAbi, UnpackAbi)]
pub struct TonTokenDetails {
    #[abi(with = "address_only_hash")]
    pub root_address: UInt256,
    #[abi(with = "uint256_bytes")]
    pub wallet_public_key: UInt256,
    #[abi(with = "address_only_hash")]
    pub owner_address: UInt256,
    #[abi(uint128)]
    pub balance: u128,
    #[abi(with = "address_only_hash")]
    pub receive_callback: UInt256,
    #[abi(with = "address_only_hash")]
    pub bounced_callback: UInt256,
    #[abi(bool)]
    pub allow_non_notifiable: bool,
}

impl TonTokenDetails {
    pub fn make_params_tuple() -> ton_abi::ParamType {
        TupleBuilder::new()
            .arg("root_address", ton_abi::ParamType::Address)
            .arg("wallet_public_key", ton_abi::ParamType::Uint(256))
            .arg("owner_address", ton_abi::ParamType::Address)
            .arg("balance", ton_abi::ParamType::Uint(128))
            .arg("receive_callback", ton_abi::ParamType::Address)
            .arg("bounced_callback", ton_abi::ParamType::Address)
            .arg("allow_non_notifiable", ton_abi::ParamType::Bool)
            .build()
    }
}
