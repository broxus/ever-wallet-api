use anyhow::Result;
use nekoton_abi::*;

pub use self::models::*;
use crate::utils::*;

pub mod ton_token_wallet_contract;

mod models;

pub struct TonTokenWalletContract<'a>(pub &'a ExistingContract);

impl TonTokenWalletContract<'_> {
    pub fn get_details(&self) -> Result<TonTokenDetails> {
        let function = ton_token_wallet_contract::get_details();
        let result = self.0.run_local(function, &[answer_id()])?.unpack_first()?;
        Ok(result)
    }

    pub fn get_version(&self) -> Result<u32> {
        let function = FunctionBuilder::new_responsible("getVersion")
            .default_headers()
            .out_arg("value0", ton_abi::ParamType::Uint(32))
            .build();

        let version: u32 = self
            .0
            .run_local(&function, &[answer_id()])?
            .unpack_first()?;

        Ok(version)
    }
}
