use anyhow::Result;
use nekoton::transport::models::ExistingContract;
use nekoton_abi::{ExecutionOutput, FunctionExt, GenTimings, LastTransactionId, TransactionId};
use ton_block::{Account, ShardAccount};

pub trait ExistingContractExt {
    fn from_shard_account(shard_account: &ShardAccount) -> Result<Option<ExistingContract>>;
    fn from_shard_account_opt(
        shard_account: &Option<ShardAccount>,
    ) -> Result<Option<ExistingContract>>;

    fn run_local(
        &self,
        function: &ton_abi::Function,
        input: &[ton_abi::Token],
    ) -> Result<Vec<ton_abi::Token>>;
}

impl ExistingContractExt for ExistingContract {
    fn from_shard_account(shard_account: &ShardAccount) -> Result<Option<Self>> {
        Ok(match shard_account.read_account()? {
            Account::Account(account) => Some(Self {
                account,
                timings: GenTimings::Unknown,
                last_transaction_id: LastTransactionId::Exact(TransactionId {
                    lt: shard_account.last_trans_lt(),
                    hash: *shard_account.last_trans_hash(),
                }),
            }),
            Account::AccountNone => None,
        })
    }

    fn from_shard_account_opt(shard_account: &Option<ShardAccount>) -> Result<Option<Self>> {
        match shard_account {
            Some(shard_account) => Self::from_shard_account(shard_account),
            None => Ok(None),
        }
    }

    fn run_local(
        &self,
        function: &ton_abi::Function,
        input: &[ton_abi::Token],
    ) -> Result<Vec<ton_abi::Token>> {
        let ExecutionOutput {
            tokens,
            result_code,
        } = function.run_local(&nekoton_utils::SimpleClock, self.account.clone(), input)?;

        tokens.ok_or_else(|| ExistingContractError::NonZeroResultCode(result_code).into())
    }
}

#[derive(thiserror::Error, Debug)]
enum ExistingContractError {
    #[error("Non zero result code: {}", .0)]
    NonZeroResultCode(i32),
}
