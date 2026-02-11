use anyhow::Result;
use tycho_types::{cell::HashBytes, models::ShardIdent};

use crate::models::ExistingContract;

/// Helper trait to reduce boilerplate for getting accounts from shards state
pub trait ShardAccountsMapExt {
    /// Looks for a suitable shard and tries to extract information about the contract from it
    fn find_account(&self, account: &HashBytes) -> Result<Option<ExistingContract>>;
}

impl<T> ShardAccountsMapExt for &T
where
    T: ShardAccountsMapExt,
{
    fn find_account(&self, account: &HashBytes) -> Result<Option<ExistingContract>> {
        T::find_account(self, account)
    }
}

pub fn contains_account(shard: &ShardIdent, account: &HashBytes) -> bool {
    let shard_prefix = shard.prefix();
    if shard_prefix == ShardIdent::PREFIX_FULL {
        true
    } else {
        let len = shard.prefix_len();
        let account_prefix = account_prefix(account, len as usize) >> (64 - len);
        let shard_prefix = shard_prefix >> (64 - len);
        account_prefix == shard_prefix
    }
}

pub fn account_prefix(account: &HashBytes, len: usize) -> u64 {
    debug_assert!(len <= 64);

    let account = account.as_slice();

    let mut value: u64 = 0;

    let bytes = len / 8;
    for (i, byte) in account.iter().enumerate().take(bytes) {
        value |= (*byte as u64) << (8 * (7 - i));
    }

    let remainder = len % 8;
    if remainder > 0 {
        let r = account[bytes] >> (8 - remainder);
        value |= (r as u64) << (8 * (7 - bytes) + 8 - remainder);
    }

    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_prefix() {
        let mut account_id = [0u8; 32];
        for byte in account_id.iter_mut().take(8) {
            *byte = 0xff;
        }

        let account_id = HashBytes::from(account_id);
        for i in 0..64 {
            let prefix = account_prefix(&account_id, i);
            assert_eq!(64 - prefix.trailing_zeros(), i as u32);
        }
    }

    #[test]
    fn test_contains_account() {
        let account = HashBytes::from_slice(
            &hex::decode("459b6795bf4d4c3b930c83fe7625cfee99a762e1e114c749b62bfa751b781fa5")
                .unwrap(),
        );

        let mut shards = vec![ShardIdent::new(0, ShardIdent::PREFIX_FULL).unwrap()];
        for _ in 0..4 {
            let mut new_shards = vec![];
            for shard in &shards {
                let (left, right) = shard.split().unwrap();
                new_shards.push(left);
                new_shards.push(right);
            }

            shards = new_shards;
        }

        let mut target_shard = None;
        for shard in shards {
            if !contains_account(&shard, &account) {
                continue;
            }

            if target_shard.is_some() {
                panic!("Account can't be in two shards");
            }
            target_shard = Some(shard);
        }

        assert!(matches!(target_shard, Some(shard) if shard.prefix() == 0x4800000000000000));
    }
}
