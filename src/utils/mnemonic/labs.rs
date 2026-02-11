use std::convert::TryInto;

use anyhow::Result;

use super::{Bip39MnemonicData, LANGUAGE};

pub fn derive_master_key(phrase: &str) -> Result<[u8; 64]> {
    let mnemonic = bip39::Mnemonic::from_phrase(phrase, LANGUAGE)?;
    let hd = bip39::Seed::new(&mnemonic, "");
    Ok(hd.as_bytes().try_into().unwrap())
}

pub fn derive_from_phrase(
    phrase: &str,
    mnemonic_data: Bip39MnemonicData,
) -> Result<ed25519_dalek::SigningKey> {
    let mnemonic = bip39::Mnemonic::from_phrase(phrase, LANGUAGE)?;
    let hd = bip39::Seed::new(&mnemonic, "");
    let seed_bytes = hd.as_bytes();

    let account_id = mnemonic_data.account_id;
    let derived = mnemonic_data.path.derive(seed_bytes, account_id)?;

    Ok(ed25519_dalek::SigningKey::from_bytes(&derived))
}

#[cfg(test)]
mod tests {
    use crate::utils::mnemonic::{Bip39Entropy, Bip39Path};

    use super::*;

    #[test]
    fn invalid_bip39_phrase() {
        let key = derive_from_phrase(
            "pioneer fever hazard scam install wise reform corn bubble leisure amazing note",
            Bip39MnemonicData {
                account_id: 0,
                path: Bip39Path::Ever,
                entropy: Bip39Entropy::Bits128,
            },
        );
        assert!(key.is_err());
    }

    #[test]
    fn correct_bip39_derive() {
        let signing_key = derive_from_phrase(
            "pioneer fever hazard scan install wise reform corn bubble leisure amazing note",
            Bip39MnemonicData {
                account_id: 0,
                path: Bip39Path::Ever,
                entropy: Bip39Entropy::Bits128,
            },
        )
        .unwrap();

        let key_hex =
            hex::decode("e371ef1d7266fc47b30d49dc886861598f09e2e6294d7f0520fe9aa460114e51")
                .unwrap();
        let mut key = [0u8; 32];
        key.copy_from_slice(&key_hex);

        let target_secret = ed25519_dalek::SigningKey::from_bytes(&key);

        assert_eq!(signing_key.as_bytes(), target_secret.as_bytes())
    }

    #[test]
    fn master_key_derive() {
        let ph = "pioneer fever hazard scan install wise reform corn bubble leisure amazing note";
        derive_master_key(ph).unwrap();
    }
}
