use std::fmt::Display;

use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use tycho_types::{
    cell::{Cell, HashBytes},
    models::{OrdinaryTxInfo, StdAddr, Transaction},
};

use crate::utils::{serde_address, serde_cell, serde_string};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum TokenWalletTransaction {
    IncomingTransfer(TokenIncomingTransfer),
    OutgoingTransfer(TokenOutgoingTransfer),
    SwapBack(TokenSwapBack),
    #[serde(with = "serde_string")]
    Accept(BigUint),
    #[serde(with = "serde_string")]
    TransferBounced(BigUint),
    #[serde(with = "serde_string")]
    SwapBackBounced(BigUint),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenIncomingTransfer {
    #[serde(with = "serde_string")]
    pub tokens: BigUint,
    /// Not the address of the token wallet, but the address of its owner
    #[serde(with = "serde_address")]
    pub sender_address: StdAddr,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenOutgoingTransfer {
    pub to: TransferRecipient,
    #[serde(with = "serde_string")]
    pub tokens: BigUint,
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
    pub tokens: BigUint,
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
    pub total_supply: BigUint,
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

pub struct TokenWalletContractState<'a>(pub ExecutionContext<'a>);

impl TokenWalletContractState<'_> {
    pub fn get_code_hash(&self) -> anyhow::Result<HashBytes> {
        match &self.0.account_stuff.storage.state {
            ton_block::AccountState::AccountActive { state_init, .. } => {
                let code = state_init
                    .code
                    .as_ref()
                    .ok_or(Tip3Error::WalletNotDeployed)?;
                Ok(code.repr_hash())
            }
            _ => Err(Tip3Error::WalletNotDeployed.into()),
        }
    }

    pub fn get_balance(&self, version: TokenWalletVersion) -> anyhow::Result<BigUint> {
        match version {
            TokenWalletVersion::OldTip3v4 => old_tip3::TokenWalletContract(self.0).balance(),
            TokenWalletVersion::Tip3 => tip3::TokenWalletContract(self.0).balance(),
        }
    }

    pub fn get_details(&self, version: TokenWalletVersion) -> anyhow::Result<TokenWalletDetails> {
        Ok(match version {
            TokenWalletVersion::OldTip3v4 => {
                let details = old_tip3::TokenWalletContract(self.0).get_details()?;

                TokenWalletDetails {
                    root_address: details.root_address,
                    owner_address: details.owner_address,
                    balance: details.balance,
                }
            }
            TokenWalletVersion::Tip3 => {
                let token_wallet = tip3::TokenWalletContract(self.0);
                let root_address = token_wallet.root()?;
                let balance = token_wallet.balance()?;

                let token_wallet = tip3_1::TokenWalletContract(self.0);
                let owner_address = token_wallet.owner()?;

                TokenWalletDetails {
                    root_address,
                    owner_address,
                    balance,
                }
            }
        })
    }

    pub fn get_version(&self) -> anyhow::Result<TokenWalletVersion> {
        if let Ok(true) = tip6::SidContract(self.0).supports_interfaces(&[
            tip3::token_wallet_contract::INTERFACE_ID,
            tip3_1::token_wallet_contract::INTERFACE_ID,
        ]) {
            return Ok(TokenWalletVersion::Tip3);
        }

        match old_tip3::TokenWalletContract(self.0).get_version()? {
            4 => Ok(TokenWalletVersion::OldTip3v4),
            _ => Err(Tip3Error::UnknownVersion.into()),
        }
    }
}

pub fn parse_token_transaction(
    tx: &Transaction,
    info: &OrdinaryTxInfo,
    version: TokenWalletVersion,
) -> Option<TokenWalletTransaction> {
    if info.aborted {
        return None;
    }

    let in_msg = tx.in_msg.as_ref()?.read_struct().ok()?;

    let mut body = in_msg.body()?;
    let function_id = read_function_id(&body).ok()?;

    let header = in_msg.int_header()?;

    let functions = TokenWalletFunctions::for_version(version);

    if header.bounced {
        body.move_by(32).ok()?;
        let function_id = read_function_id(&body).ok()?;
        body.move_by(32).ok()?;

        if function_id == functions.accept_transfer.input_id {
            return Some(TokenWalletTransaction::TransferBounced(
                body.get_next_u128().ok()?.into(),
            ));
        }

        if function_id == functions.accept_burn.input_id {
            Some(TokenWalletTransaction::SwapBackBounced(
                body.get_next_u128().ok()?.into(),
            ))
        } else {
            None
        }
    } else if function_id == functions.accept_mint.input_id {
        let inputs = functions.accept_mint.decode_input(body, true, false).ok()?;

        Accept::try_from((InputMessage(inputs), version))
            .map(|Accept { tokens }| TokenWalletTransaction::Accept(tokens))
            .ok()
    } else if function_id == functions.transfer_to_wallet.input_id {
        let inputs = functions
            .transfer_to_wallet
            .decode_input(body, true, false)
            .ok()?;

        TokenOutgoingTransfer::try_from((
            InputMessage(inputs),
            TransferType::ByTokenWalletAddress,
            version,
        ))
        .map(TokenWalletTransaction::OutgoingTransfer)
        .ok()
    } else if function_id == functions.transfer.input_id {
        let inputs = functions.transfer.decode_input(body, true, false).ok()?;

        TokenOutgoingTransfer::try_from((
            InputMessage(inputs),
            TransferType::ByOwnerWalletAddress,
            version,
        ))
        .map(TokenWalletTransaction::OutgoingTransfer)
        .ok()
    } else if function_id == functions.accept_transfer.input_id {
        let inputs = functions
            .accept_transfer
            .decode_input(body, true, false)
            .ok()?;

        TokenIncomingTransfer::try_from((InputMessage(inputs), version))
            .map(TokenWalletTransaction::IncomingTransfer)
            .ok()
    } else if function_id == functions.burn.input_id {
        let inputs = functions.burn.decode_input(body, true, false).ok()?;

        TokenSwapBack::try_from((InputMessage(inputs), version))
            .map(TokenWalletTransaction::SwapBack)
            .ok()
    } else {
        None
    }
}
