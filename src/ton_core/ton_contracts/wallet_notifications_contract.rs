use nekoton_abi::*;
use once_cell::sync::OnceCell;

pub fn notify_wallet_deployed() -> &'static ton_abi::Function {
    static FUNCTION: OnceCell<ton_abi::Function> = OnceCell::new();
    FUNCTION.get_or_init(|| {
        FunctionBuilder::new("notifyWalletDeployed")
            .time_header()
            .expire_header()
            .input("root", ton_abi::ParamType::Address)
            .build()
    })
}
