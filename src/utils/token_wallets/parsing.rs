use std::{convert::TryFrom, sync::OnceLock};

use anyhow::Result;
use tycho_types::{
    abi::Function,
    cell::Load,
    models::{MsgInfo, OrdinaryTxInfo, OwnedMessage, Transaction},
};

use crate::utils::{
    token_wallets::{
        self,
        models::{
            TokenIncomingTransfer, TokenOutgoingTransfer, TokenSwapBack, TokenWalletTransaction,
            TokenWalletVersion, TransferRecipient,
        },
    },
    ton_wallet::multisig::UnpackerError,
    InputMessage,
};

pub fn parse_token_transaction(
    tx: &Transaction,
    info: &OrdinaryTxInfo,
    version: TokenWalletVersion,
) -> Option<TokenWalletTransaction> {
    if info.aborted {
        return None;
    }

    let in_msg_cell = tx.in_msg.as_ref()?;

    let Ok(mut slice) = in_msg_cell.as_slice() else {
        return None;
    };
    let Ok(in_msg) = OwnedMessage::load_from(&mut slice) else {
        return None;
    };

    let body = in_msg.body;
    let Ok(mut body_slice) = body.1.as_slice() else {
        return None;
    };

    let Ok(function_id) = body_slice.get_u32(0) else {
        return None;
    };

    let MsgInfo::Int(info) = in_msg.info else {
        return None;
    };

    let Ok(functions) = TokenWalletFunctions::for_version(version) else {
        return None;
    };

    if info.bounced {
        body_slice.skip_first(32, 0).ok()?;
        let Ok(function_id) = body_slice.get_u32(0) else {
            return None;
        };
        body_slice.skip_first(32, 0).ok()?;

        if function_id == functions.accept_transfer.input_id {
            return Some(TokenWalletTransaction::TransferBounced(
                body_slice.load_u128().ok()?,
            ));
        }

        if function_id == functions.accept_burn.input_id {
            Some(TokenWalletTransaction::SwapBackBounced(
                body_slice.load_u128().ok()?,
            ))
        } else {
            None
        }
    } else if function_id == functions.accept_mint.input_id {
        let inputs = functions
            .accept_mint
            .decode_internal_input(body_slice)
            .ok()?;

        Accept::try_from((InputMessage(inputs), version))
            .map(|Accept { tokens }| TokenWalletTransaction::Accept(tokens))
            .ok()
    } else if function_id == functions.transfer_to_wallet.input_id {
        let inputs = functions
            .transfer_to_wallet
            .decode_internal_input(body_slice)
            .ok()?;

        TokenOutgoingTransfer::try_from((
            InputMessage(inputs),
            TransferType::ByTokenWalletAddress,
            version,
        ))
        .map(TokenWalletTransaction::OutgoingTransfer)
        .ok()
    } else if function_id == functions.transfer.input_id {
        let inputs = functions.transfer.decode_internal_input(body_slice).ok()?;

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
            .decode_internal_input(body_slice)
            .ok()?;

        TokenIncomingTransfer::try_from((InputMessage(inputs), version))
            .map(TokenWalletTransaction::IncomingTransfer)
            .ok()
    } else if function_id == functions.burn.input_id {
        let inputs = functions.burn.decode_internal_input(body_slice).ok()?;

        TokenSwapBack::try_from((InputMessage(inputs), version))
            .map(TokenWalletTransaction::SwapBack)
            .ok()
    } else {
        None
    }
}

struct TokenWalletFunctions {
    // Incoming
    accept_mint: &'static Function,
    // Incoming
    transfer: &'static Function,
    // Incoming
    transfer_to_wallet: &'static Function,
    // Incoming
    accept_transfer: &'static Function,
    // Incoming
    burn: &'static Function,
    // Outgoing
    accept_burn: &'static Function,
}

impl TokenWalletFunctions {
    pub fn for_version(
        version: TokenWalletVersion,
    ) -> Result<&'static TokenWalletFunctions, UnpackerError> {
        match version {
            TokenWalletVersion::OldTip3v4 => Err(UnpackerError::InvalidAbi),
            TokenWalletVersion::Tip3 => {
                static IDS: OnceLock<TokenWalletFunctions> = OnceLock::new();
                Ok(IDS.get_or_init(|| Self {
                    accept_mint: token_wallets::accept_mint(),
                    transfer: token_wallets::transfer(),
                    transfer_to_wallet: token_wallets::transfer_to_wallet(),
                    accept_transfer: token_wallets::accept_transfer(),
                    burn: token_wallets::burnable::burn(),
                    accept_burn: token_wallets::accept_burn(),
                }))
            }
        }
    }
}

impl TryFrom<(InputMessage, TokenWalletVersion)> for TokenSwapBack {
    type Error = UnpackerError;

    fn try_from((value, version): (InputMessage, TokenWalletVersion)) -> Result<Self, Self::Error> {
        Ok(match version {
            TokenWalletVersion::OldTip3v4 => {
                return Err(UnpackerError::InvalidAbi);
            }
            TokenWalletVersion::Tip3 => {
                let input = token_wallets::burnable::BurnInputs::unpack(value.0)
                    .map_err(|_| UnpackerError::InvalidAbi)?;

                Self {
                    tokens: input.amount,
                    callback_address: input.callback_to,
                    callback_payload: input.payload,
                }
            }
        })
    }
}

struct Accept {
    tokens: u128,
}

impl TryFrom<(InputMessage, TokenWalletVersion)> for Accept {
    type Error = UnpackerError;

    fn try_from((value, version): (InputMessage, TokenWalletVersion)) -> Result<Self, Self::Error> {
        Ok(match version {
            TokenWalletVersion::OldTip3v4 => {
                return Err(UnpackerError::InvalidAbi);
            }
            TokenWalletVersion::Tip3 => {
                let input = token_wallets::AcceptMintInputs::unpack(value.0)
                    .map_err(|_| UnpackerError::InvalidAbi)?;
                Self {
                    tokens: input.amount,
                }
            }
        })
    }
}

enum TransferType {
    ByOwnerWalletAddress,
    ByTokenWalletAddress,
}

impl TryFrom<(InputMessage, TransferType, TokenWalletVersion)> for TokenOutgoingTransfer {
    type Error = UnpackerError;

    fn try_from(
        (value, transfer_type, version): (InputMessage, TransferType, TokenWalletVersion),
    ) -> Result<Self, Self::Error> {
        Ok(match version {
            TokenWalletVersion::OldTip3v4 => {
                return Err(UnpackerError::InvalidAbi);
            }
            TokenWalletVersion::Tip3 => {
                match transfer_type {
                    // "transfer"
                    TransferType::ByOwnerWalletAddress => {
                        let input = token_wallets::TransferInputs::unpack(value.0)
                            .map_err(|_| UnpackerError::InvalidAbi)?;
                        Self {
                            to: TransferRecipient::OwnerWallet(input.recipient),
                            tokens: input.amount,
                            payload: input.payload,
                        }
                    }
                    // "transferToWallet"
                    TransferType::ByTokenWalletAddress => {
                        let input = token_wallets::TransferToWalletInputs::unpack(value.0)
                            .map_err(|_| UnpackerError::InvalidAbi)?;
                        Self {
                            to: TransferRecipient::TokenWallet(input.recipient_token_wallet),
                            tokens: input.amount,
                            payload: input.payload,
                        }
                    }
                }
            }
        })
    }
}

impl TryFrom<(InputMessage, TokenWalletVersion)> for TokenIncomingTransfer {
    type Error = UnpackerError;

    fn try_from((value, version): (InputMessage, TokenWalletVersion)) -> Result<Self, Self::Error> {
        Ok(match version {
            TokenWalletVersion::OldTip3v4 => {
                return Err(UnpackerError::InvalidAbi);
            }
            TokenWalletVersion::Tip3 => {
                let input = token_wallets::AcceptTransferInputs::unpack(value.0)
                    .map_err(|_| UnpackerError::InvalidAbi)?;
                Self {
                    tokens: input.amount,
                    sender_address: input.sender,
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {

    use tycho_types::{boc::Boc, cell::Load, models::TxInfo};

    use crate::utils::token_wallets::models::TokenWalletVersion;

    use super::*;

    fn parse_transaction(data: &str) -> (Transaction, OrdinaryTxInfo) {
        let binding = Boc::decode_base64(data).unwrap();
        let mut cell = binding.as_slice().unwrap();
        let transaction = Transaction::load_from(&mut cell).unwrap();
        let info = match transaction.load_info().unwrap() {
            TxInfo::Ordinary(info) => info,
            _ => panic!(),
        };
        (transaction, info)
    }

    #[test]
    fn test_parse_wallet_v3_token_transfer_with_payload() {
        let (transaction, description) = parse_transaction("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");

        println!("tx: {transaction:#?}");

        println!("description: {description:#?}");

        let parsed = parse_token_transaction(&transaction, &description, TokenWalletVersion::Tip3);
        println!("parsed tx: {parsed:#?}");
    }
}
