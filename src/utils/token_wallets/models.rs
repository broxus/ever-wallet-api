use std::fmt::Display;

use nekoton_core::contracts::blockchain_context::BlockchainAccount;
use nekoton_core::contracts::function_ext::ExecutionOutput;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use tycho_types::{
    abi::{AbiValue, NamedAbiValue},
    cell::Cell,
    models::{AnyAddr, StdAddr},
};

use crate::utils::{serde_address, serde_cell, serde_string};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum TokenWalletTransaction {
    IncomingTransfer(TokenIncomingTransfer),
    OutgoingTransfer(TokenOutgoingTransfer),
    SwapBack(TokenSwapBack),
    #[serde(with = "serde_string")]
    Accept(u128),
    #[serde(with = "serde_string")]
    TransferBounced(u128),
    #[serde(with = "serde_string")]
    SwapBackBounced(u128),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenIncomingTransfer {
    #[serde(with = "serde_string")]
    pub tokens: u128,
    /// Not the address of the token wallet, but the address of its owner
    #[serde(with = "serde_address")]
    pub sender_address: StdAddr,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenOutgoingTransfer {
    pub to: TransferRecipient,
    #[serde(with = "serde_string")]
    pub tokens: u128,
    /// token transfer payload
    #[serde(with = "serde_cell")]
    pub payload: Cell,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "data")]
pub enum TransferRecipient {
    #[serde(with = "serde_address")]
    OwnerWallet(StdAddr),
    #[serde(with = "serde_address")]
    TokenWallet(StdAddr),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenSwapBack {
    #[serde(with = "serde_string")]
    pub tokens: u128,
    #[serde(with = "serde_address")]
    pub callback_address: StdAddr,
    /// ETH address or something else
    #[serde(with = "serde_cell")]
    pub callback_payload: Cell,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum TokenWalletVersion {
    /// Third iteration of token wallets, but with fixed bugs
    /// [implementation](https://github.com/broxus/ton-eth-bridge-token-contracts/tree/74905260499d79cf7cb0d89a6eb572176fc1fcd5)
    OldTip3v4,
    /// Latest iteration with completely new standard
    /// [implementation](https://github.com/broxus/ton-eth-bridge-token-contracts/tree/9168190f218fd05a64269f5f24295c69c4840d94)
    Tip3,
}

impl Display for TokenWalletVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TokenWalletVersion::OldTip3v4 => "OldTip3v4",
            TokenWalletVersion::Tip3 => "Tip3",
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RootTokenContractDetails {
    /// Token ecosystem version
    pub version: TokenWalletVersion,
    /// Full currency name
    pub name: String,
    /// Short currency name
    pub symbol: String,
    /// Decimals
    pub decimals: u8,
    /// Root owner contract address. Used as proxy address in Tip3v1
    #[serde(with = "serde_address")]
    pub owner_address: StdAddr,
    #[serde(with = "serde_string")]
    pub total_supply: u128,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenWalletDetails {
    /// Linked root token contract address
    #[serde(with = "serde_address")]
    pub root_address: StdAddr,

    /// Owner wallet address
    #[serde(with = "serde_address")]
    pub owner_address: StdAddr,

    #[serde(with = "serde_string")]
    pub balance: u128,
}

#[derive(thiserror::Error, Debug)]
pub enum Tip3Error {
    #[error("Unknown version")]
    UnknownVersion,
    #[error("Wallet not deployed")]
    WalletNotDeployed,
}

pub struct TokenWalletContractState<'a>(pub &'a mut BlockchainAccount);

impl TokenWalletContractState<'_> {
    pub fn get_balance(&mut self, version: TokenWalletVersion) -> anyhow::Result<u128> {
        match version {
            TokenWalletVersion::OldTip3v4 => Err(Tip3Error::UnknownVersion.into()),
            TokenWalletVersion::Tip3 => TokenWalletContract(self.0).balance(),
        }
    }

    pub fn get_details(
        &mut self,
        version: TokenWalletVersion,
    ) -> anyhow::Result<TokenWalletDetails> {
        Ok(match version {
            TokenWalletVersion::OldTip3v4 => {
                return Err(Tip3Error::UnknownVersion.into());
            }
            TokenWalletVersion::Tip3 => {
                let mut token_wallet = TokenWalletContract(self.0);
                let root_address = token_wallet.root()?;
                let balance = token_wallet.balance()?;

                let mut token_wallet = TokenWalletContract(self.0);
                let owner_address = token_wallet.owner()?;

                TokenWalletDetails {
                    root_address,
                    owner_address,
                    balance,
                }
            }
        })
    }

    pub fn get_version(&mut self) -> anyhow::Result<TokenWalletVersion> {
        if let Ok(true) = SidContract(self.0).supports_interfaces(&[0x2a4ac43e, 0x4F479FA3]) {
            Ok(TokenWalletVersion::Tip3)
        } else {
            Err(Tip3Error::UnknownVersion.into())
        }
    }
}

pub struct RootTokenContractState<'a>(pub &'a mut BlockchainAccount);

impl RootTokenContractState<'_> {
    /// Calculates token wallet address
    pub fn get_wallet_address(
        &mut self,
        version: TokenWalletVersion,
        owner: &StdAddr,
    ) -> anyhow::Result<StdAddr> {
        match version {
            TokenWalletVersion::OldTip3v4 => {
                anyhow::bail!(Tip3Error::UnknownVersion)
            }
            TokenWalletVersion::Tip3 => RootTokenContract(self.0).wallet_of(owner.clone()),
        }
    }

    /// Tries to guess version and retrieve details
    pub fn guess_details(&mut self) -> anyhow::Result<RootTokenContractDetails> {
        if let Ok(true) = SidContract(self.0).supports_interfaces(&[0x4371D8ED, 0x0b1fd263]) {
            return self.get_details(TokenWalletVersion::Tip3);
        }

        self.get_details(TokenWalletVersion::Tip3)
    }

    /// Retrieve details using specified version
    pub fn get_details(
        &mut self,
        version: TokenWalletVersion,
    ) -> anyhow::Result<RootTokenContractDetails> {
        Ok(match version {
            TokenWalletVersion::OldTip3v4 => {
                anyhow::bail!(Tip3Error::UnknownVersion)
            }
            TokenWalletVersion::Tip3 => {
                let mut root_contract = RootTokenContract(self.0);
                let name = root_contract.name()?;
                let symbol = root_contract.symbol()?;
                let decimals = root_contract.decimals()?;
                let total_supply = root_contract.total_supply()?;
                let owner_address = root_contract.root_owner()?;

                RootTokenContractDetails {
                    version,
                    name,
                    symbol,
                    decimals,
                    owner_address,
                    total_supply,
                }
            }
        })
    }
}

pub struct RootTokenContract<'a>(pub &'a mut BlockchainAccount);

impl RootTokenContract<'_> {
    pub fn name(&mut self) -> anyhow::Result<String> {
        let inputs = [AbiValue::uint(32, 0u32).named("answerId")];
        let ExecutionOutput { values, exit_code } =
            self.0.run_local_responsible(super::name(), &inputs)?;

        if exit_code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to get name with exit code {exit_code}"
            ));
        }

        let result = values
            .first()
            .ok_or_else(|| anyhow::anyhow!("Failed to get name"))?
            .clone();

        let AbiValue::String(name) = result.value else {
            return Err(anyhow::anyhow!("Failed to get name"));
        };

        Ok(name)
    }

    pub fn symbol(&mut self) -> anyhow::Result<String> {
        let inputs = [AbiValue::uint(32, 0u32).named("answerId")];
        let ExecutionOutput { values, exit_code } =
            self.0.run_local_responsible(super::symbol(), &inputs)?;

        if exit_code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to get symbol with exit code {exit_code}"
            ));
        }

        let result = values
            .first()
            .ok_or_else(|| anyhow::anyhow!("Failed to get symbol"))?
            .clone();

        let AbiValue::String(symbol) = result.value else {
            return Err(anyhow::anyhow!("Failed to get symbol"));
        };

        Ok(symbol)
    }

    pub fn decimals(&mut self) -> anyhow::Result<u8> {
        let inputs = [AbiValue::uint(32, 0u32).named("answerId")];
        let ExecutionOutput { values, exit_code } =
            self.0.run_local_responsible(super::decimals(), &inputs)?;

        if exit_code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to get decimals with exit code {exit_code}"
            ));
        }

        let result = values
            .first()
            .ok_or_else(|| anyhow::anyhow!("Failed to get decimals"))?
            .clone();

        let AbiValue::Uint(_, decimals) = result.value else {
            return Err(anyhow::anyhow!("Failed to get decimals"));
        };

        Ok(decimals.to_u8().unwrap())
    }

    pub fn total_supply(&mut self) -> anyhow::Result<u128> {
        let inputs = [AbiValue::uint(32, 0u32).named("answerId")];
        let ExecutionOutput { values, exit_code } = self
            .0
            .run_local_responsible(super::total_supply(), &inputs)?;

        if exit_code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to get total_supply with exit code {exit_code}"
            ));
        }

        let result = values
            .first()
            .ok_or_else(|| anyhow::anyhow!("Failed to get total_supply"))?
            .clone();

        let AbiValue::Uint(_, total_supply) = result.value else {
            return Err(anyhow::anyhow!("Failed to get total_supply"));
        };

        Ok(total_supply.to_u128().unwrap())
    }

    pub fn wallet_code(&mut self) -> anyhow::Result<Cell> {
        let inputs = [AbiValue::uint(32, 0u32).named("answerId")];
        let ExecutionOutput { values, exit_code } = self
            .0
            .run_local_responsible(super::wallet_code(), &inputs)?;

        if exit_code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to get wallet_code with exit code {exit_code}"
            ));
        }

        let result = values
            .first()
            .ok_or_else(|| anyhow::anyhow!("Failed to get wallet_code"))?
            .clone();

        let AbiValue::Cell(wallet_code) = result.value else {
            return Err(anyhow::anyhow!("Failed to get wallet_code"));
        };

        Ok(wallet_code)
    }

    pub fn root_owner(&mut self) -> anyhow::Result<StdAddr> {
        let inputs = [AbiValue::uint(32, 0u32).named("answerId")];
        let ExecutionOutput { values, exit_code } =
            self.0.run_local_responsible(super::root_owner(), &inputs)?;

        if exit_code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to get root_owner with exit code {exit_code}"
            ));
        }

        let result = values
            .first()
            .ok_or_else(|| anyhow::anyhow!("Failed to get root_owner"))?
            .clone();

        let AbiValue::Address(root_owner) = result.value else {
            return Err(anyhow::anyhow!("Failed to get root_owner"));
        };

        let AnyAddr::Std(root_owner) = &*root_owner else {
            return Err(anyhow::anyhow!("Failed to get root_owner"));
        };

        Ok(root_owner.clone())
    }

    pub fn wallet_of(&mut self, owner: StdAddr) -> anyhow::Result<StdAddr> {
        let inputs = [
            AbiValue::uint(32, 0u32).named("answerId"),
            AbiValue::address(owner).named("owner"),
        ];
        let ExecutionOutput { values, exit_code } =
            self.0.run_local_responsible(super::wallet_of(), &inputs)?;

        if exit_code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to get wallet_of with exit code {exit_code}"
            ));
        }

        let result = values
            .first()
            .ok_or_else(|| anyhow::anyhow!("Failed to get wallet_of"))?
            .clone();

        let AbiValue::Address(wallet_of) = result.value else {
            return Err(anyhow::anyhow!("Failed to get wallet_of"));
        };
        let AnyAddr::Std(wallet_of) = &*wallet_of else {
            return Err(anyhow::anyhow!("Failed to get wallet_of"));
        };

        Ok(wallet_of.clone())
    }
}

pub struct TokenWalletContract<'a>(pub &'a mut BlockchainAccount);

impl TokenWalletContract<'_> {
    pub fn owner(&mut self) -> anyhow::Result<StdAddr> {
        let inputs = [AbiValue::uint(32, 0u32).named("answerId")];
        let ExecutionOutput { values, exit_code } =
            self.0.run_local_responsible(super::owner(), &inputs)?;

        if exit_code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to get owner with exit code {exit_code}"
            ));
        }

        let result = values
            .first()
            .ok_or_else(|| anyhow::anyhow!("Failed to get owner"))?
            .clone();

        let AbiValue::Address(owner) = result.value else {
            return Err(anyhow::anyhow!("Failed to get owner"));
        };
        let AnyAddr::Std(owner) = &*owner else {
            return Err(anyhow::anyhow!("Failed to get owner"));
        };

        Ok(owner.clone())
    }
    pub fn root(&mut self) -> anyhow::Result<StdAddr> {
        let inputs = [AbiValue::uint(32, 0u32).named("answerId")];
        let ExecutionOutput { values, exit_code } =
            self.0.run_local_responsible(super::root(), &inputs)?;

        if exit_code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to get root with exit code {exit_code}"
            ));
        }

        let result = values
            .first()
            .ok_or_else(|| anyhow::anyhow!("Failed to get root"))?
            .clone();

        let AbiValue::Address(root) = result.value else {
            return Err(anyhow::anyhow!("Failed to get owrootner"));
        };
        let AnyAddr::Std(root) = &*root else {
            return Err(anyhow::anyhow!("Failed to get root"));
        };

        Ok(root.clone())
    }

    pub fn balance(&mut self) -> anyhow::Result<u128> {
        let inputs = [AbiValue::uint(32, 0u32).named("answerId")];
        let ExecutionOutput { values, exit_code } =
            self.0.run_local_responsible(super::balance(), &inputs)?;

        if exit_code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to get balance with exit code {exit_code}"
            ));
        }

        let result = values
            .first()
            .ok_or_else(|| anyhow::anyhow!("Failed to get balance"))?
            .clone();

        let AbiValue::Uint(_, balance) = result.value else {
            return Err(anyhow::anyhow!("Failed to get balance"));
        };

        Ok(balance.to_u128().unwrap())
    }
}

pub struct SidContract<'a>(pub &'a mut BlockchainAccount);

impl SidContract<'_> {
    pub fn supports_interfaces(&mut self, interfaces: &[u32]) -> anyhow::Result<bool> {
        let mut inputs: Option<[NamedAbiValue; 2]> = None;

        for &interface in interfaces {
            let inputs = match &mut inputs {
                Some(inputs) => {
                    inputs[1] = make_interface_id(interface);
                    inputs
                }
                None => inputs.insert([
                    AbiValue::uint(32, 0u32).named("answerId"),
                    make_interface_id(interface),
                ]),
            };

            let ExecutionOutput { values, exit_code } = self
                .0
                .run_local_responsible(super::supports_interface(), inputs.as_ref())?;

            if exit_code != 0 {
                return Err(anyhow::anyhow!(
                    "Failed to get name with exit code {exit_code}"
                ));
            }

            let result = values
                .first()
                .ok_or_else(|| anyhow::anyhow!("Failed to get name"))?
                .clone();

            let AbiValue::Bool(b) = result.value else {
                return Err(anyhow::anyhow!("Failed to get name"));
            };

            if !b {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub fn supports_interface(&mut self, interface: u32) -> anyhow::Result<bool> {
        let inputs = [
            AbiValue::uint(32, 0u32).named("answerId"),
            make_interface_id(interface),
        ];
        let ExecutionOutput { values, exit_code } = self
            .0
            .run_local_responsible(super::supports_interface(), &inputs)?;

        if exit_code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to get supports_interface with exit code {exit_code}"
            ));
        }

        let result = values
            .first()
            .ok_or_else(|| anyhow::anyhow!("Failed to get supports_interface"))?
            .clone();

        let AbiValue::Bool(supports_interface) = result.value else {
            return Err(anyhow::anyhow!("Failed to get supports_interface"));
        };

        Ok(supports_interface)
    }
}

fn make_interface_id(interface: u32) -> NamedAbiValue {
    AbiValue::uint(32, interface).named("interfaceID")
}
