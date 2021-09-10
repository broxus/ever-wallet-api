use nekoton_abi::*;
use ton_block::MsgAddressInt;

#[derive(Debug, Clone, PackAbiPlain, UnpackAbiPlain, KnownParamTypePlain)]
pub struct NotifyWalletDeployed {
    #[abi(address)]
    pub root: MsgAddressInt,
}
