use std::convert::TryFrom;

use anyhow::Result;
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer};
use nekoton::core::models::Expiration;
use nekoton::core::ton_wallet::MultisigType;
use nekoton::crypto::SignedMessage;

use crate::models::sqlx::AddressDb;
use crate::prelude::ServiceError;

pub fn prepare_deploy(address: AddressDb) -> Result<SignedMessage> {
    let data = PrepareDeploy::try_from(address)?;

    let unsigned_message = nekoton::core::ton_wallet::multisig::prepare_deploy(
        &data.public_key,
        data.multisig_type,
        data.workchain,
        data.expiration,
        &data.owners,
        data.req_confirms,
    )?;

    let key_pair = Keypair {
        secret: data.secret,
        public: data.public_key,
    };

    let signature = key_pair.sign(unsigned_message.hash());
    unsigned_message.sign(&signature.to_bytes())
}

const DEFAULT_EXPIRATION_TIMEOUT: u32 = 60;
const DEFAULT_MULTISIG_TYPE: MultisigType = MultisigType::SafeMultisigWallet;

struct PrepareDeploy {
    public_key: PublicKey,
    secret: SecretKey,
    multisig_type: MultisigType,
    workchain: i8,
    expiration: Expiration,
    owners: Vec<PublicKey>,
    req_confirms: u8,
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
                .map_err(|_| ServiceError::WrongInput(format!("Invalid private key")))?;

        let mut owners = Vec::new();
        {
            let owners_str = serde_json::from_value::<Vec<String>>(
                item.custodians_public_keys.unwrap_or_default(),
            )?;
            for owner in owners_str {
                let owner_public_key =
                    PublicKey::from_bytes(&hex::decode(owner.as_bytes()).map_err(|_| {
                        ServiceError::WrongInput(format!(
                            "Invalid custodian public key `{}`",
                            owner
                        ))
                    })?)
                    .map_err(|_| {
                        ServiceError::WrongInput(format!(
                            "Invalid custodian public key `{}`",
                            owner
                        ))
                    })?;
                owners.push(owner_public_key);
            }
        }

        let workchain = item.workchain_id as i8;
        let multisig_type = DEFAULT_MULTISIG_TYPE;
        let expiration = Expiration::Timeout(DEFAULT_EXPIRATION_TIMEOUT);
        let req_confirms = item.confirmations.unwrap_or_default() as u8;

        Ok(PrepareDeploy {
            public_key,
            secret,
            multisig_type,
            workchain,
            expiration,
            owners,
            req_confirms,
        })
    }
}
