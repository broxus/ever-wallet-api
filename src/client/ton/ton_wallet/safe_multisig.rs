use std::convert::TryFrom;

use anyhow::Result;
use nekoton::core::models::Expiration;
use nekoton::core::ton_wallet::MultisigType;
use nekoton::crypto::UnsignedMessage;
use nekoton_utils::TrustMe;

use super::PrepareDeploy;

const MULTISIG_TYPE: MultisigType = MultisigType::SafeMultisigWallet;

pub fn prepare_deploy(data: &PrepareDeploy) -> Result<Box<dyn UnsignedMessage>> {
    nekoton::core::ton_wallet::multisig::prepare_deploy(
        &data.public_key,
        MULTISIG_TYPE,
        data.workchain,
        data.expiration,
        &data.owners.trust_me(),
        data.req_confirms.trust_me(),
    )
}
