use tycho_types::{
    abi::{AbiType, Function, NamedAbiType},
    cell::Cell,
    models::StdAddr,
};

use crate::utils::declare_function;

pub const INTERFACE_ID: u32 = 0x2a4ac43e;

pub mod models;

/// Get token wallet owner address
///
/// # Type
/// Internal responsible getter
///
/// # Inputs
/// * `answerId: uint32` - responsible answer id
///
/// # Outputs
/// * `owner: address` - token wallet owner address
///
pub fn owner() -> &'static Function {
    declare_function! {
        name: "owner",
        inputs: vec![AbiType::Uint(32).named("answerId")],
        outputs: vec![AbiType::Address.named("owner")],
    }
}

#[derive(Debug, Clone)]
pub struct TransferInputs {
    pub amount: u128,
    pub recipient: StdAddr,
    pub deploy_wallet_value: u128,
    pub remaining_gas_to: StdAddr,
    pub notify: bool,
    pub payload: Cell,
}

impl TransferInputs {
    fn abi_type() -> Vec<NamedAbiType> {
        vec![
            AbiType::Uint(128).named("amount"),
            AbiType::Address.named("recipient"),
            AbiType::Uint(128).named("deployWalletValue"),
            AbiType::Address.named("remainingGasTo"),
            AbiType::Bool.named("notify"),
            AbiType::Cell.named("payload"),
        ]
    }
}

/// Transfer tokens and optionally deploy token wallet for the recipient
///
/// # Type
/// Internal method
///
/// # Inputs
/// * `amount: uint128` - amount of tokens to transfer
/// * `recipient: address` - tokens recipient address
/// * `deployWalletValue: uint128` - how much EVERs to attach to the token wallet deploy
/// * `remainingGasTo: address` - remaining gas receiver
/// * `notify: bool` - notify receiver on incoming transfer
/// * `payload: cell` - arbitrary payload
///
pub fn transfer() -> &'static Function {
    declare_function! {
        name: "transfer",
        inputs: TransferInputs::abi_type(),
        outputs: Vec::new(),
    }
}

#[derive(Debug, Clone)]
pub struct TransferToWalletInputs {
    pub amount: u128,
    pub recipient_token_wallet: StdAddr,
    pub remaining_gas_to: StdAddr,
    pub notify: bool,
    pub payload: Cell,
}

impl TransferToWalletInputs {
    fn abi_type() -> Vec<NamedAbiType> {
        vec![
            AbiType::Uint(128).named("amount"),
            AbiType::Address.named("recipientTokenWallet"),
            AbiType::Address.named("remainingGasTo"),
            AbiType::Bool.named("notify"),
            AbiType::Cell.named("payload"),
        ]
    }
}

/// Transfer tokens using token wallet address
///
/// # Type
/// Internal method
///
/// # Inputs
/// * `amount: uint128` - amount of tokens to transfer
/// * `recipientTokenWallet: address` - recipient token wallet
/// * `remainingGasTo: address` - remaining gas receiver
/// * `notify: bool` - notify receiver on incoming transfer
/// * `payload: cell` - arbitrary payload
///
pub fn transfer_to_wallet() -> &'static Function {
    declare_function! {
        name: "transferToWallet",
        inputs: TransferToWalletInputs::abi_type(),
        outputs: Vec::new(),
    }
}

#[derive(Debug, Clone)]
pub struct AcceptTransferInputs {
    pub amount: u128,
    pub sender: StdAddr,
    pub remaining_gas_to: StdAddr,
    pub notify: bool,
    pub payload: Cell,
}

impl AcceptTransferInputs {
    fn abi_type() -> Vec<NamedAbiType> {
        vec![
            AbiType::Uint(128).named("amount"),
            AbiType::Address.named("sender"),
            AbiType::Address.named("remainingGasTo"),
            AbiType::Bool.named("notify"),
            AbiType::Cell.named("payload"),
        ]
    }
}

/// Callback for transfer operation
///
/// # Type
/// Internal method
///
/// # Inputs
/// * `amount: uint128` - how much tokens to receive
/// * `sender: address` - token wallet owner address
/// * `remainingGasTo` -
/// * `notify` -
///
///
/// TODO: fill docs
///
pub fn accept_transfer() -> &'static Function {
    declare_function! {
        function_id: 0x67A0B95F,
        name: "acceptTransfer",
        inputs: AcceptTransferInputs::abi_type(),
        outputs: Vec::new(),
    }
}

#[derive(Debug, Clone)]
pub struct AcceptMintInputs {
    pub amount: u128,
    pub remaining_gas_to: StdAddr,
    pub notify: bool,
    pub payload: Cell,
}

impl AcceptMintInputs {
    fn abi_type() -> Vec<NamedAbiType> {
        vec![
            AbiType::Uint(128).named("amount"),
            AbiType::Address.named("remainingGasTo"),
            AbiType::Bool.named("notify"),
            AbiType::Cell.named("payload"),
        ]
    }
}

/// Accept minted tokens from root
///
/// # Type
/// Internal method
///
/// # Inputs
/// * `amount: uint128` - how much tokens to receive
/// * `remainingGasTo: address` - remaining gas receiver
/// * `notify: bool` - notify receiver on incoming mint
/// * `payload: cell` - arbitrary payload
///
pub fn accept_mint() -> &'static Function {
    declare_function! {
        function_id: 0x4384F298,
        name: "acceptMint",
        inputs: AcceptMintInputs::abi_type(),
        outputs: Vec::new(),
    }
}

pub mod burnable {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct BurnInputs {
        pub amount: u128,
        pub remaining_gas_to: StdAddr,
        pub callback_to: StdAddr,
        pub payload: Cell,
    }

    impl BurnInputs {
        fn abi_type() -> Vec<NamedAbiType> {
            vec![
                AbiType::Uint(128).named("amount"),
                AbiType::Address.named("remainingGasTo"),
                AbiType::Address.named("callbackTo"),
                AbiType::Cell.named("payload"),
            ]
        }
    }

    /// TODO: fill docs
    pub fn burn() -> &'static Function {
        declare_function! {
            name: "burn",
            inputs: BurnInputs::abi_type(),
            outputs: Vec::new(),
        }
    }
}

/// Mint tokens to recipient with deploy wallet optional
///
/// # Type
/// Internal method
///
/// # Inputs
/// * `amount: uint128` - how much tokens to mint
/// * `recipient: address` - minted tokens owner address
/// * `deployWalletValue: uint128` - how much EVERs to send to wallet on deployment
/// * `remainingGasTo: address` - address where to send excess gas
/// * `notify: bool` - whether to notify the recipient
/// * `payload: cell` - arbitrary payload
///
pub fn mint() -> &'static Function {
    declare_function! {
        name: "mint",
        inputs: vec![
            AbiType::Uint(128).named("amount"),
            AbiType::Address.named("recipient"),
            AbiType::Uint(128).named("deployWalletValue"),
            AbiType::Address.named("remainingGasTo"),
            AbiType::Bool.named("notify"),
            AbiType::Cell.named("payload"),
        ],
        outputs: Vec::new(),
    }
}
