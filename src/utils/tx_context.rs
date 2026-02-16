use anyhow::Result;
use nekoton_core::contracts::blockchain_context::BlockchainContext;
use tokio::sync::oneshot;
use tycho_types::{
    abi::{Function, NamedAbiValue},
    cell::{CellSlice, HashBytes},
    models::{BlockId, MsgInfo, OrdinaryTxInfo, OwnedMessage, Transaction},
};

use crate::models::ExistingContract;

pub trait ReadFromTransaction: Sized {
    fn read_from_transaction(ctx: &TxContext<'_>, state: HandleTransactionStatusTx)
        -> Option<Self>;
}
pub trait ReadFromState: Sized {
    fn read_from_state(ctx: &StateContext<'_>, state: HandleTransactionStatusTx) -> Self;
}

#[derive(Copy, Clone)]
pub struct StateContext<'a> {
    pub block_id: &'a BlockId,
}

#[derive(Copy, Clone)]
pub struct TxContext<'a> {
    pub block_info_gen_utime: u32,
    pub block_hash: &'a HashBytes,
    pub account: &'a HashBytes,
    pub transaction_hash: &'a HashBytes,
    pub transaction_info: &'a OrdinaryTxInfo,
    pub transaction: &'a Transaction,
    pub in_msg: &'a OwnedMessage,
    pub token_transaction: &'a Option<crate::utils::token_wallets::models::TokenWalletTransaction>,
    pub token_state: &'a Option<ExistingContract>,
    pub blockchain_context: &'a Option<BlockchainContextWrapper>,
}

#[derive(Clone)]
pub struct BlockchainContextWrapper {
    pub blockchain_context: BlockchainContext,
}

impl std::fmt::Debug for BlockchainContextWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockchainContextWrapper")
            .field("blockchain_context", &"context")
            .finish()
    }
}

impl TxContext<'_> {
    pub fn in_msg_internal(&self) -> Option<&OwnedMessage> {
        if matches!(self.in_msg.info, MsgInfo::Int(_)) {
            Some(self.in_msg)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn in_msg_external(&self) -> Option<&OwnedMessage> {
        if matches!(self.in_msg.info, MsgInfo::ExtIn(_)) {
            Some(self.in_msg)
        } else {
            None
        }
    }
    #[allow(dead_code)]
    pub fn find_function_output(&self, function: &Function) -> Result<Option<Vec<NamedAbiValue>>> {
        for message in self.transaction.iter_out_msgs() {
            let message = message?;
            // Skip all messages except external outgoing
            if !matches!(message.info, MsgInfo::ExtOut(_)) {
                continue;
            }

            let function_id = message.body.get_u32(0)?;
            if function_id != function.output_id {
                continue;
            }

            match function.decode_output(message.body) {
                Ok(tokens) => {
                    return Ok(Some(tokens));
                }
                Err(_) => continue,
            }
        }
        Ok(None)
    }

    #[allow(dead_code)]
    pub fn iterate_events<F>(&self, mut f: F)
    where
        F: FnMut(u32, CellSlice<'_>),
    {
        for message in self.transaction.iter_out_msgs() {
            let Ok(message) = message else { continue };
            // Skip all messages except external outgoing
            if !matches!(message.info, MsgInfo::ExtOut(_)) {
                continue;
            }

            if let Ok(function_id) = message.body.get_u32(0) {
                f(function_id, message.body)
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum HandleTransactionStatus {
    Success,
    Fail,
}
pub type HandleTransactionStatusTx = oneshot::Sender<HandleTransactionStatus>;
pub type HandleTransactionStatusRx = oneshot::Receiver<HandleTransactionStatus>;
