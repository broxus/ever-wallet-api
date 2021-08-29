/*use anyhow::Result;
use nekoton::core::InternalMessage;
use nekoton_abi::{BigUint128, BigUint256, MessageBuilder};
use nekoton_utils::TrustMe;
use ton_block::MsgAddressInt;

pub fn prepare_deploy(
    owner: MsgAddressInt,
    root_token_contract: MsgAddressInt,
) -> Result<InternalMessage> {
    const INITIAL_BALANCE: u64 = 100_000_000; // 0.1 TON
    const ATTACHED_AMOUNT: u64 = 500_000_000; // 0.5 TON

    let (function, input) = MessageBuilder::new(
        nekoton_contracts::abi::root_token_contract_v3(),
        "deployEmptyWallet",
    )
    .trust_me()
    .arg(BigUint128(INITIAL_BALANCE.into()))
    .arg(BigUint256(Default::default()))
    .arg(&owner)
    .arg(&owner)
    .build();

    let body = function
        .encode_input(&Default::default(), &input, true, None)?
        .into();

    Ok(InternalMessage {
        source: Some(owner.clone()),
        destination: root_token_contract,
        amount: ATTACHED_AMOUNT,
        bounce: true,
        body,
    })
}
*/
