use tycho_types::cell::HashBytes;

#[derive(Clone, Debug)]
pub struct ExistingContract {
    pub account: tycho_types::models::Account,
    pub last_transaction_hash: HashBytes,
}
