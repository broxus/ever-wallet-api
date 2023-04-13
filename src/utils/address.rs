use anyhow::Result;
use ed25519_dalek::PublicKey;

use crate::{models::AccountType, services::TonServiceError};

pub fn validate_account_type(
    account_type: &Option<AccountType>,
    custodians: Option<i32>,
    confirmations: Option<i32>,
) -> Result<(Option<i32>, Option<i32>)> {
    let (custodians, confirmations) = match account_type.unwrap_or_default() {
        AccountType::SafeMultisig => (
            Some(custodians.unwrap_or(1)),
            Some(confirmations.unwrap_or(1)),
        ),
        AccountType::HighloadWallet | AccountType::Wallet => (None, None),
    };

    if let (Some(custodians), Some(confirmations)) = (custodians, confirmations) {
        if confirmations > custodians {
            return Err(
                TonServiceError::WrongInput("Invalid number of confirmations".to_string()).into(),
            );
        }
    }

    Ok((custodians, confirmations))
}

pub fn validate_public_keys(
    custodians_public_keys: &[&str],
    account_public_key: &str,
    account_type: &Option<AccountType>,
) -> Result<(Option<Vec<String>>, String)> {
    let account_key = PublicKey::from_bytes(
        &hex::decode(account_public_key)
            .map_err(|_| TonServiceError::WrongInput("Invalid custodian".to_string()))?,
    )
    .map_err(|_| TonServiceError::WrongInput("Invalid custodian".to_string()))?;
    let account_key_encoded = hex::encode(account_key.to_bytes());

    let custodians_public_keys = match account_type.unwrap_or_default() {
        AccountType::SafeMultisig => {
            let mut custodians = Vec::with_capacity(custodians_public_keys.len());
            for key in custodians_public_keys {
                custodians.push(
                    PublicKey::from_bytes(&hex::decode(key).map_err(|_| {
                        TonServiceError::WrongInput("Invalid custodian".to_string())
                    })?)
                    .map_err(|_| TonServiceError::WrongInput("Invalid custodian".to_string()))?,
                );
            }

            if !custodians.iter().any(|&k| k == account_key) {
                custodians.push(account_key);
            }
            let custodians = custodians
                .into_iter()
                .map(|key| hex::encode(key.to_bytes()))
                .collect();

            Some(custodians)
        }
        AccountType::HighloadWallet | AccountType::Wallet => None,
    };

    Ok((custodians_public_keys, account_key_encoded))
}
