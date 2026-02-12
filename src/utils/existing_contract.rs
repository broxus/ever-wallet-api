use anyhow::Result;
use nekoton_core::contracts::blockchain_context::BlockchainContext;
use nekoton_core::contracts::function_ext::{ExecutionOutput, FunctionExt};
use tycho_types::{
    abi::{Function, NamedAbiValue},
    models::ShardAccount,
};

use crate::models::ExistingContract;

pub trait ExistingContractExt {
    fn from_shard_account(shard_account: &ShardAccount) -> Result<Option<ExistingContract>>;
    fn from_shard_account_opt(
        shard_account: &Option<ShardAccount>,
    ) -> Result<Option<ExistingContract>>;

    fn run_local(
        &mut self,
        function: &Function,
        input: &[NamedAbiValue],
        responsible: bool,
        context: &mut BlockchainContext,
    ) -> Result<Vec<NamedAbiValue>>;
}

impl ExistingContractExt for ExistingContract {
    fn from_shard_account(shard_account: &ShardAccount) -> Result<Option<Self>> {
        if let Some(account) = shard_account.load_account()? {
            Ok(Some(Self {
                account,
                last_transaction_hash: shard_account.last_trans_hash,
            }))
        } else {
            Ok(None)
        }
    }

    fn from_shard_account_opt(shard_account: &Option<ShardAccount>) -> Result<Option<Self>> {
        match shard_account {
            Some(shard_account) => Self::from_shard_account(shard_account),
            None => Ok(None),
        }
    }

    fn run_local(
        &mut self,
        function: &Function,
        input: &[NamedAbiValue],
        responsible: bool,
        context: &mut BlockchainContext,
    ) -> Result<Vec<NamedAbiValue>> {
        let ExecutionOutput { values, exit_code } =
            function.run_local(&mut self.account, input, responsible, context)?;

        if exit_code != 0 {
            return Err(ExistingContractError::NonZeroResultCode(exit_code).into());
        }

        Ok(values)
    }
}

#[derive(thiserror::Error, Debug)]
enum ExistingContractError {
    #[error("Non zero result code: {}", .0)]
    NonZeroResultCode(i32),
}
