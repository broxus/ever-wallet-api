use ton_abi::{Param, ParamType};

use crate::utils::declare_function;

pub fn notify_wallet_deployed() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_0,
        name: "notifyWalletDeployed",
        inputs: vec![Param::new("root", ParamType::Address)],
        outputs: Vec::new(),
    }
}

pub fn depool_on_round_complete() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_0,
        name: "onRoundComplete",
        inputs: vec![
            Param::new("roundId", ParamType::Uint(64)),
            Param::new("reward", ParamType::Uint(64)),
            Param::new("ordinaryStake", ParamType::Uint(64)),
            Param::new("vestingStake", ParamType::Uint(64)),
            Param::new("lockStake", ParamType::Uint(64)),
            Param::new("reinvest", ParamType::Bool),
            Param::new("reason", ParamType::Uint(8)),
        ],
        outputs: Vec::new(),
    }
}

pub fn depool_receive_answer() -> &'static ton_abi::Function {
    declare_function! {
        abi: v2_0,
        name: "receiveAnswer",
        inputs: vec![
            Param::new("errcode", ParamType::Uint(32)),
            Param::new("comment", ParamType::Uint(64)),
        ],
        outputs: Vec::new(),
    }
}
