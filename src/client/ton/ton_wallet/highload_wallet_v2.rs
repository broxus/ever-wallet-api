use std::convert::TryFrom;

use anyhow::Result;
use nekoton::crypto::UnsignedMessage;

use super::PrepareDeploy;

pub fn prepare_deploy(data: &PrepareDeploy) -> Result<Box<dyn UnsignedMessage>> {
    nekoton::core::ton_wallet::highload_wallet_v2::prepare_deploy(
        &data.public_key,
        data.workchain,
        data.expiration,
    )
}
