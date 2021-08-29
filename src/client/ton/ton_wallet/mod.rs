use std::convert::TryFrom;

use anyhow::Result;
use ed25519_dalek::{PublicKey, SecretKey};
use nekoton::core::models::Expiration;
use nekoton::core::ton_wallet::MultisigType;

use crate::models::account_enums::AccountType;
use crate::models::sqlx::AddressDb;
use crate::prelude::ServiceError;

pub const DEFAULT_EXPIRATION_TIMEOUT: u32 = 60;
pub const MULTISIG_TYPE: MultisigType = MultisigType::SafeMultisigWallet;

pub struct PrepareDeploy {
    pub public_key: PublicKey,
    pub secret: SecretKey,
    pub workchain: i8,
    pub expiration: Expiration,
    pub owners: Option<Vec<PublicKey>>,
    pub req_confirms: Option<u8>,
    pub account_type: AccountType,
}

impl TryFrom<AddressDb> for PrepareDeploy {
    type Error = anyhow::Error;

    fn try_from(item: AddressDb) -> Result<Self> {
        let public_key =
            PublicKey::from_bytes(&hex::decode(item.public_key.as_bytes()).map_err(|_| {
                ServiceError::WrongInput(format!("Invalid public key `{}`", item.public_key))
            })?)
            .map_err(|_| {
                ServiceError::WrongInput(format!("Invalid public key `{}`", item.public_key))
            })?;

        let secret =
            SecretKey::from_bytes(&hex::decode(item.private_key.as_bytes()).unwrap_or_default())
                .map_err(|_| ServiceError::WrongInput("Invalid private key".to_string()))?;

        let workchain = item.workchain_id as i8;
        let expiration = Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT);

        let mut owners = None;
        let mut req_confirms = None;
        if let AccountType::SafeMultisig = item.account_type {
            let mut owners_pubkey = Vec::new();
            let owners_str = serde_json::from_value::<Vec<String>>(
                item.custodians_public_keys.unwrap_or_default(),
            )?;
            for owner in owners_str {
                let owner_public_key =
                    PublicKey::from_bytes(&hex::decode(owner.as_bytes()).map_err(|_| {
                        ServiceError::WrongInput(format!("Invalid owner public key `{}`", owner))
                    })?)
                    .map_err(|_| {
                        ServiceError::WrongInput(format!("Invalid owner public key `{}`", owner))
                    })?;
                owners_pubkey.push(owner_public_key);
            }
            owners = Some(owners_pubkey);

            req_confirms = Some(
                item.confirmations
                    .ok_or(ServiceError::WrongInput("Invalid request".to_string()))?
                    as u8,
            );
        }

        let account_type = item.account_type;

        Ok(PrepareDeploy {
            public_key,
            secret,
            workchain,
            expiration,
            owners,
            req_confirms,
            account_type,
        })
    }
}
