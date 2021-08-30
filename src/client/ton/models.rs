use crate::models::account_enums::AccountType;

pub enum AddressDeploy {
    HighloadWallet(DeployWallet),
    WalletV3(DeployWallet),
    SafeMultisig(DeploySafeMultisigWallet),
}

pub struct DeployWallet {
    pub public_key: Vec<u8>,
    pub secret: Vec<u8>,
    pub workchain: i8,
}

pub struct DeploySafeMultisigWallet {
    pub public_key: Vec<u8>,
    pub secret: Vec<u8>,
    pub workchain: i8,
    pub owners: Vec<Vec<u8>>,
    pub req_confirms: u8,
}
