use nekoton_abi::*;
use once_cell::sync::OnceCell;

use super::models::*;

pub fn get_details() -> &'static ton_abi::Function {
    static FUNCTION: OnceCell<ton_abi::Function> = OnceCell::new();
    FUNCTION.get_or_init(|| {
        FunctionBuilder::new_responsible("getDetails")
            .default_headers()
            .out_arg("value0", TonTokenDetails::make_params_tuple())
            .build()
    })
}
