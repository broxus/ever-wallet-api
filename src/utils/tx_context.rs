use everscale_types::{
    cell::HashBytes,
    models::{BlockId, BlockInfo, Message, MsgInfo, OrdinaryTxInfo, Transaction},
};
use nekoton::transport::models::ExistingContract;
use tokio::sync::oneshot;

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
    pub block_info: &'a BlockInfo,
    pub block_hash: &'a HashBytes,
    pub account: &'a HashBytes,
    pub transaction_hash: &'a HashBytes,
    pub transaction_info: &'a OrdinaryTxInfo,
    pub transaction: &'a Transaction,
    pub in_msg: &'a Message<'a>,
    pub token_transaction: &'a Option<nekoton::core::models::TokenWalletTransaction>,
    pub token_state: &'a Option<ExistingContract>,
}

impl TxContext<'_> {
    pub fn in_msg_internal(&self) -> Option<&Message<'_>> {
        if matches!(self.in_msg.info, MsgInfo::Int(_)) {
            Some(self.in_msg)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn in_msg_external(&self) -> Option<&Message<'_>> {
        if matches!(self.in_msg.info, MsgInfo::ExtIn(_)) {
            Some(self.in_msg)
        } else {
            None
        }
    }

    // #[allow(dead_code)]
    // pub fn find_function_output(
    //     &self,
    //     function: &Function,
    // ) -> Option<Vec<Token>> {
    //     for message in self.transaction.iter_out_msgs(){
    //         let Ok(message) = message else  {
    //             continue;
    //         };
    //         // Skip all messages except external outgoing
    //         if !matches!(message.info, MsgInfo::ExtOut(_)) {
    //             continue;
    //         }

    //         // Handle body if it exists
    //         let function_id = nekoton_abi::read_function_id(&message.body)?;
    //         if function_id != function.output_id {
    //             return Ok(true);
    //         }

    //         match function.decode_output(message.body, false) {
    //             Ok(tokens) => {
    //                 return Some(tokens);
    //             }
    //             Err(_) => {},
    //         }
    //     }
    //     None
    // }

    // #[allow(dead_code)]
    // pub fn iterate_events<F>(&self, mut f: F)
    // where
    //     F: FnMut(u32, SliceData),
    // {
    //      for message in self.transaction.iter_out_msgs(){
    //         let Ok(message) = message else  {
    //             continue ;
    //         };
    //         // Skip all messages except external outgoing
    //         if !matches!(message.info, MsgInfo::ExtOut(_)) {
    //             continue;
    //         }

    //         // Parse function id
    //         if let Ok(function_id) = nekoton_abi::read_function_id(&message.body) {
    //             f(function_id, message.body)
    //         }
    //     }
    // }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum HandleTransactionStatus {
    Success,
    Fail,
}

pub type HandleTransactionStatusTx = oneshot::Sender<HandleTransactionStatus>;
pub type HandleTransactionStatusRx = oneshot::Receiver<HandleTransactionStatus>;
