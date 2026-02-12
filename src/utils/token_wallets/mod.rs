use anyhow::{anyhow, Result};
use num_traits::ToPrimitive;
use tycho_types::{
    abi::{AbiType, AbiValue, Function, NamedAbiType, NamedAbiValue},
    cell::Cell,
    models::{AnyAddr, StdAddr},
};

use crate::utils::declare_function;

pub const INTERFACE_ID: u32 = 0x2a4ac43e;

pub mod models;
pub mod parsing;

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

    pub fn unpack(values: Vec<NamedAbiValue>) -> Result<Self> {
        if values.len() != 6 {
            return Err(anyhow!("Invalid number of arguments"));
        }
        let amount_abi_value = &values[0];
        let recipient_abi_value = &values[1];
        let deploy_wallet_value_abi_value = &values[2];
        let remaining_gas_to_abi_value = &values[3];
        let notify_abi_value = &values[4];
        let payload_abi_value = &values[5];

        if &*amount_abi_value.name != "amount"
            && amount_abi_value.value.get_type() != AbiType::Uint(128)
        {
            return Err(anyhow!("Invalid amount"));
        }
        let AbiValue::Uint(_, amount) = &amount_abi_value.value else {
            return Err(anyhow!("Invalid amount"));
        };

        if &*recipient_abi_value.name != "recipient"
            && recipient_abi_value.value.get_type() != AbiType::Address
        {
            return Err(anyhow!("Invalid recipient"));
        }
        let AbiValue::Address(recipient) = &recipient_abi_value.value else {
            return Err(anyhow!("Invalid recipient"));
        };

        let AnyAddr::Std(recipient) = *recipient.clone() else {
            return Err(anyhow!("Invalid recipient"));
        };

        if &*deploy_wallet_value_abi_value.name != "deployWalletValue"
            && deploy_wallet_value_abi_value.value.get_type() != AbiType::Uint(128)
        {
            return Err(anyhow!("Invalid deployWalletValue"));
        }
        let AbiValue::Uint(_, deploy_wallet_value) = &deploy_wallet_value_abi_value.value else {
            return Err(anyhow!("Invalid deployWalletValue"));
        };

        if &*remaining_gas_to_abi_value.name != "remainingGasTo"
            && remaining_gas_to_abi_value.value.get_type() != AbiType::Address
        {
            return Err(anyhow!("Invalid remainingGasTo"));
        }
        let AbiValue::Address(remaining_gas_to) = &remaining_gas_to_abi_value.value else {
            return Err(anyhow!("Invalid remainingGasTo"));
        };

        let AnyAddr::Std(remaining_gas_to) = *remaining_gas_to.clone() else {
            return Err(anyhow!("Invalid remainingGasTo"));
        };

        if &*notify_abi_value.name != "notify" && notify_abi_value.value.get_type() != AbiType::Bool
        {
            return Err(anyhow!("Invalid notify"));
        }
        let AbiValue::Bool(notify) = &notify_abi_value.value else {
            return Err(anyhow!("Invalid notify"));
        };

        if &*payload_abi_value.name != "payload"
            && payload_abi_value.value.get_type() != AbiType::Cell
        {
            return Err(anyhow!("Invalid payload"));
        }
        let AbiValue::Cell(payload) = &payload_abi_value.value else {
            return Err(anyhow!("Invalid payload"));
        };

        Ok(Self {
            amount: amount.to_u128().unwrap(),
            recipient: recipient,
            deploy_wallet_value: deploy_wallet_value.to_u128().unwrap(),
            remaining_gas_to: remaining_gas_to,
            notify: *notify,
            payload: payload.clone(),
        })
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

    pub fn unpack(values: Vec<NamedAbiValue>) -> Result<Self> {
        if values.len() != 5 {
            return Err(anyhow!("Invalid number of arguments"));
        }
        let amount_abi_value = &values[0];
        let recipient_token_wallet_abi_value = &values[1];
        let remaining_gas_to_abi_value = &values[2];
        let notify_abi_value = &values[3];
        let payload_abi_value = &values[4];

        if &*amount_abi_value.name != "amount"
            && amount_abi_value.value.get_type() != AbiType::Uint(128)
        {
            return Err(anyhow!("Invalid amount"));
        }
        let AbiValue::Uint(_, amount) = &amount_abi_value.value else {
            return Err(anyhow!("Invalid amount"));
        };

        if &*recipient_token_wallet_abi_value.name != "recipientTokenWallet"
            && recipient_token_wallet_abi_value.value.get_type() != AbiType::Address
        {
            return Err(anyhow!("Invalid recipientTokenWallet"));
        }
        let AbiValue::Address(recipient_token_wallet) = &recipient_token_wallet_abi_value.value
        else {
            return Err(anyhow!("Invalid recipientTokenWallet"));
        };

        let AnyAddr::Std(recipient_token_wallet) = *recipient_token_wallet.clone() else {
            return Err(anyhow!("Invalid recipientTokenWallet"));
        };

        if &*remaining_gas_to_abi_value.name != "remainingGasTo"
            && remaining_gas_to_abi_value.value.get_type() != AbiType::Address
        {
            return Err(anyhow!("Invalid remainingGasTo"));
        }
        let AbiValue::Address(remaining_gas_to) = &remaining_gas_to_abi_value.value else {
            return Err(anyhow!("Invalid remainingGasTo"));
        };

        let AnyAddr::Std(remaining_gas_to) = *remaining_gas_to.clone() else {
            return Err(anyhow!("Invalid remainingGasTo"));
        };

        if &*notify_abi_value.name != "notify" && notify_abi_value.value.get_type() != AbiType::Bool
        {
            return Err(anyhow!("Invalid notify"));
        }
        let AbiValue::Bool(notify) = &notify_abi_value.value else {
            return Err(anyhow!("Invalid notify"));
        };

        if &*payload_abi_value.name != "payload"
            && payload_abi_value.value.get_type() != AbiType::Cell
        {
            return Err(anyhow!("Invalid payload"));
        }
        let AbiValue::Cell(payload) = &payload_abi_value.value else {
            return Err(anyhow!("Invalid payload"));
        };

        Ok(Self {
            amount: amount.to_u128().unwrap(),
            recipient_token_wallet: recipient_token_wallet,
            remaining_gas_to: remaining_gas_to,
            notify: *notify,
            payload: payload.clone(),
        })
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

    pub fn unpack(values: Vec<NamedAbiValue>) -> Result<Self> {
        if values.len() != 5 {
            return Err(anyhow!("Invalid number of arguments"));
        }
        let amount_abi_value = &values[0];
        let sender_abi_value = &values[1];
        let remaining_gas_to_abi_value = &values[2];
        let notify_abi_value = &values[3];
        let payload_abi_value = &values[4];

        if &*amount_abi_value.name != "amount"
            && amount_abi_value.value.get_type() != AbiType::Uint(128)
        {
            return Err(anyhow!("Invalid amount"));
        }
        let AbiValue::Uint(_, amount) = &amount_abi_value.value else {
            return Err(anyhow!("Invalid amount"));
        };

        if &*sender_abi_value.name != "sender"
            && sender_abi_value.value.get_type() != AbiType::Address
        {
            return Err(anyhow!("Invalid sender"));
        }
        let AbiValue::Address(sender) = &sender_abi_value.value else {
            return Err(anyhow!("Invalid sender"));
        };

        let AnyAddr::Std(sender) = *sender.clone() else {
            return Err(anyhow!("Invalid sender"));
        };

        if &*remaining_gas_to_abi_value.name != "remainingGasTo"
            && remaining_gas_to_abi_value.value.get_type() != AbiType::Address
        {
            return Err(anyhow!("Invalid remainingGasTo"));
        }
        let AbiValue::Address(remaining_gas_to) = &remaining_gas_to_abi_value.value else {
            return Err(anyhow!("Invalid remainingGasTo"));
        };

        let AnyAddr::Std(remaining_gas_to) = *remaining_gas_to.clone() else {
            return Err(anyhow!("Invalid remainingGasTo"));
        };

        if &*notify_abi_value.name != "notify" && notify_abi_value.value.get_type() != AbiType::Bool
        {
            return Err(anyhow!("Invalid notify"));
        }
        let AbiValue::Bool(notify) = &notify_abi_value.value else {
            return Err(anyhow!("Invalid notify"));
        };

        if &*payload_abi_value.name != "payload"
            && payload_abi_value.value.get_type() != AbiType::Cell
        {
            return Err(anyhow!("Invalid payload"));
        }
        let AbiValue::Cell(payload) = &payload_abi_value.value else {
            return Err(anyhow!("Invalid payload"));
        };

        Ok(Self {
            amount: amount.to_u128().unwrap(),
            sender: sender,
            remaining_gas_to: remaining_gas_to,
            notify: *notify,
            payload: payload.clone(),
        })
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

    pub fn unpack(values: Vec<NamedAbiValue>) -> Result<Self> {
        if values.len() != 4 {
            return Err(anyhow!("Invalid number of arguments"));
        }
        let amount_abi_value = &values[0];
        let remaining_gas_to_abi_value = &values[1];
        let notify_abi_value = &values[2];
        let payload_abi_value = &values[3];

        if &*amount_abi_value.name != "amount"
            && amount_abi_value.value.get_type() != AbiType::Uint(128)
        {
            return Err(anyhow!("Invalid amount"));
        }
        let AbiValue::Uint(_, amount) = &amount_abi_value.value else {
            return Err(anyhow!("Invalid amount"));
        };

        if &*remaining_gas_to_abi_value.name != "remainingGasTo"
            && remaining_gas_to_abi_value.value.get_type() != AbiType::Address
        {
            return Err(anyhow!("Invalid remainingGasTo"));
        }
        let AbiValue::Address(remaining_gas_to) = &remaining_gas_to_abi_value.value else {
            return Err(anyhow!("Invalid remainingGasTo"));
        };

        let AnyAddr::Std(remaining_gas_to) = *remaining_gas_to.clone() else {
            return Err(anyhow!("Invalid remainingGasTo"));
        };

        if &*notify_abi_value.name != "notify" && notify_abi_value.value.get_type() != AbiType::Bool
        {
            return Err(anyhow!("Invalid notify"));
        }
        let AbiValue::Bool(notify) = &notify_abi_value.value else {
            return Err(anyhow!("Invalid notify"));
        };

        if &*payload_abi_value.name != "payload"
            && payload_abi_value.value.get_type() != AbiType::Cell
        {
            return Err(anyhow!("Invalid payload"));
        }
        let AbiValue::Cell(payload) = &payload_abi_value.value else {
            return Err(anyhow!("Invalid payload"));
        };

        Ok(Self {
            amount: amount.to_u128().unwrap(),
            remaining_gas_to: remaining_gas_to,
            notify: *notify,
            payload: payload.clone(),
        })
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
    use num_traits::ToPrimitive;
    use tycho_types::{
        abi::{AbiValue, NamedAbiValue},
        models::AnyAddr,
    };

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

        pub fn unpack(values: Vec<NamedAbiValue>) -> Result<Self> {
            if values.len() != 4 {
                return Err(anyhow!("Invalid number of arguments"));
            }
            let amount_abi_value = &values[0];
            let remaining_gas_to_abi_value = &values[1];
            let callback_to_abi_value = &values[2];
            let payload_abi_value = &values[3];

            if &*amount_abi_value.name != "amount"
                && amount_abi_value.value.get_type() != AbiType::Uint(128)
            {
                return Err(anyhow!("Invalid amount"));
            }
            let AbiValue::Uint(_, amount) = &amount_abi_value.value else {
                return Err(anyhow!("Invalid amount"));
            };

            if &*remaining_gas_to_abi_value.name != "remainingGasTo"
                && remaining_gas_to_abi_value.value.get_type() != AbiType::Address
            {
                return Err(anyhow!("Invalid remainingGasTo"));
            }
            let AbiValue::Address(remaining_gas_to) = &remaining_gas_to_abi_value.value else {
                return Err(anyhow!("Invalid remainingGasTo"));
            };

            let AnyAddr::Std(remaining_gas_to) = *remaining_gas_to.clone() else {
                return Err(anyhow!("Invalid remainingGasTo"));
            };

            if &*callback_to_abi_value.name != "callbackTo"
                && callback_to_abi_value.value.get_type() != AbiType::Address
            {
                return Err(anyhow!("Invalid callbackTo"));
            }
            let AbiValue::Address(callback_to) = &callback_to_abi_value.value else {
                return Err(anyhow!("Invalid callbackTo"));
            };

            let AnyAddr::Std(callback_to) = *callback_to.clone() else {
                return Err(anyhow!("Invalid callback_to"));
            };

            if &*payload_abi_value.name != "payload"
                && payload_abi_value.value.get_type() != AbiType::Cell
            {
                return Err(anyhow!("Invalid payload"));
            }
            let AbiValue::Cell(payload) = &payload_abi_value.value else {
                return Err(anyhow!("Invalid payload"));
            };

            Ok(Self {
                amount: amount.to_u128().unwrap(),
                remaining_gas_to: remaining_gas_to,
                callback_to: callback_to,
                payload: payload.clone(),
            })
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

#[derive(Debug, Clone)]
pub struct AcceptBurnInputs {
    pub amount: u128,
    pub wallet_owner: StdAddr,
    pub remaining_gas_to: StdAddr,
    pub callback_to: StdAddr,
    pub payload: Cell,
}

impl AcceptBurnInputs {
    fn abi_type() -> Vec<NamedAbiType> {
        vec![
            AbiType::Uint(128).named("amount"),
            AbiType::Address.named("walletOwner"),
            AbiType::Address.named("remainingGasTo"),
            AbiType::Address.named("callbackTo"),
            AbiType::Cell.named("payload"),
        ]
    }
}

/// Called by token wallet on burn
///
/// # Type
/// Internal method
///
/// # Inputs
/// * `amount: uint128` - amount of tokens
/// * `walletOwner: address` - token wallet owner
/// * `remainingGasTo: address` - address where to send excess gas
/// * `callbackTo: address` - address where to send callback
/// * `payload: cell` - arbitrary payload
///
pub fn accept_burn() -> &'static Function {
    declare_function! {
        function_id: 0x192B51B1,
        name: "acceptBurn",
        inputs: AcceptBurnInputs::abi_type(),
        outputs: Vec::new(),
    }
}

/// Returns the token wallet code.
///
/// # Type
/// Responsible getter method
///
/// # Inputs
/// * `answerId: uint32` - responsible answer id
///
/// # Outputs
/// * `walletCode: cell`
///
pub fn wallet_code() -> &'static Function {
    declare_function! {
        name: "walletCode",
        inputs: vec![
            AbiType::Uint(32).named("answerId"),],
        outputs: vec![
            AbiType::Cell.named("walletCode")],
    }
}

/// Returns the total token supply.
///
/// # Type
/// Responsible getter method
///
/// # Inputs
/// * `answerId: uint32` - responsible answer id
///
/// # Outputs
/// * `totalSupply: string`
///
pub fn total_supply() -> &'static Function {
    declare_function! {
        name: "totalSupply",
        inputs: vec![
            AbiType::Uint(32).named("answerId"),
            ],
        outputs: vec![
            AbiType::Uint(128).named("totalSupply"),
            ],
    }
}

/// Returns the name of the token - e.g. `MyToken`.
///
/// # Type
/// Responsible getter method
///
/// # Inputs
/// * `answerId: uint32` - responsible answer id
///
/// # Outputs
/// * `name: string`
///
pub fn name() -> &'static Function {
    declare_function! {
        name: "name",
        inputs: vec![
            AbiType::Uint(32).named("answerId"),
            ],
        outputs: vec![
            AbiType::String.named("name"),
            ],
    }
}

/// Returns the symbol of the token. E.g. "HIX".
///
/// # Type
/// Responsible getter method
///
/// # Inputs
/// * `answerId: uint32` - responsible answer id
///
/// # Outputs
/// * `symbol: string`
///
pub fn symbol() -> &'static Function {
    declare_function! {
        name: "symbol",
        inputs: vec![
            AbiType::Uint(32).named("answerId"),
            ],
        outputs: vec![

            AbiType::String.named("symbol"),
            ],
    }
}

/// Returns the number of decimals the token uses - e.g. 8,
/// means to divide the token amount by 100000000 to get its user representation.
///
/// # Type
/// Responsible getter method
///
/// # Inputs
/// * `answerId: uint32` - responsible answer id
///
/// # Outputs
/// * `decimals: uint8`
///
pub fn decimals() -> &'static Function {
    declare_function! {
        name: "decimals",
        inputs: vec![
            AbiType::Uint(32).named("answerId"),
            ],
        outputs: vec![

            AbiType::Uint(8).named("decimals"),],
    }
}

/// A contract that is compliant with TIP6 shall implement the following interface
///
/// # Type
/// Responsible getter method
///
/// # Inputs
/// * `answerId: uint32` - responsible answer id
/// * `interfaceID: bytes4` - interface ID
///
/// # Outputs
/// * `name: string`
///
pub fn supports_interface() -> &'static Function {
    declare_function! {
        name: "supportsInterface",
        inputs: vec![
            AbiType::Uint(32).named("answerId"),
            AbiType::Uint(32).named("interfaceID"),
        ],
        outputs: vec![
            AbiType::Bool.named("supports"),
            ],
    }
}

/// Returns the token root address.
///
/// # Type
/// Responsible getter method
///
/// # Inputs
/// * `answerId: uint32` - responsible answer id
///
/// # Outputs
/// * `root: address`
///
pub fn root() -> &'static Function {
    declare_function! {
        name: "root",
        inputs: vec![
            AbiType::Uint(32).named("answerId"),
            ],
        outputs: vec![
            AbiType::Address.named("root"),
            ],
    }
}

/// Returns the token wallet balance.
///
/// # Type
/// Responsible getter method
///
/// # Inputs
/// * `answerId: uint32` - responsible answer id
///
/// # Outputs
/// * `balance: uint128`
///
pub fn balance() -> &'static Function {
    declare_function! {
        name: "balance",
        inputs: vec![
            AbiType::Uint(32).named("answerId"),
            ],
        outputs: vec![

            AbiType::Uint(128).named("balance"),
            ],
    }
}

/// Get root owner
///
/// # Type
/// Responsible getter method
///
/// # Inputs
/// * `answerId: uint32` - responsible answer id
///
/// # Outputs
/// * `owner: address` - owner wallet address
///
pub fn root_owner() -> &'static Function {
    declare_function! {
        name: "rootOwner",
        inputs: vec![
            AbiType::Uint(32).named("answerId"),
            ],
        outputs: vec![

            AbiType::Address.named("rootOwner"),],
    }
}

/// Derive `TokenWallet` address from owner address
///
/// # Type
/// Responsible getter method
///
/// # Inputs
/// * `answerId: uint32` - responsible answer id
/// * `owner: address` - owner address
///
/// # Outputs
/// * `walletAddress: address` - owner wallet address
///
pub fn wallet_of() -> &'static Function {
    declare_function! {
        name: "walletOf",
        inputs: vec![
            AbiType::Uint(32).named("answerId"),
            AbiType::Address.named("owner"),
        ],
        outputs: vec![
            AbiType::Address.named("walletAddress"),
            ],
    }
}
