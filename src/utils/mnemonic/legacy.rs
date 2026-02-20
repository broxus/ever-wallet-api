use anyhow::Error;
use pbkdf2::{
    hmac::{Hmac, Mac},
    pbkdf2_hmac,
};

use super::LANGUAGE;

pub fn derive_from_phrase(phrase: &str) -> Result<ed25519_dalek::SigningKey, Error> {
    const PBKDF_ITERATIONS: u32 = 100_000;
    const SALT: &[u8] = b"TON default seed";

    let wordmap = LANGUAGE.wordmap();
    let mut word_count = 0;
    for word in phrase.split_whitespace() {
        word_count += 1;
        if word_count > 24 {
            anyhow::bail!("Expected 24 words")
        }

        wordmap.get_bits(word)?;
    }
    if word_count != 24 {
        anyhow::bail!("Expected 24 words")
    }

    let password = Hmac::<sha2::Sha512>::new_from_slice(phrase.as_bytes())
        .unwrap()
        .finalize()
        .into_bytes();

    let mut res = [0; 512 / 8];
    pbkdf2_hmac::<sha2::Sha512>(&password, SALT, PBKDF_ITERATIONS, &mut res);
    Ok(ed25519_dalek::SigningKey::from_bytes(
        &res[0..32].try_into().unwrap(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    use base64::{engine::general_purpose, Engine as _};

    #[test]
    fn correct_legacy_derive() {
        let keypair = derive_from_phrase("unaware face erupt ceiling frost shiver crumble know party before brisk skirt fence boat powder copy plastic until butter fluid property concert say verify").unwrap();
        let expected = "o0kpHL39KRq0KX11zZ0/sCwJL66t+gA4vnfuwBjhAWU=";
        let pub_expecteed = "lHW4ZS8QvCHcgR4uChD7QJWU2kf5JRMtUnZ2p1GSZjg=";
        assert_eq!(
            general_purpose::STANDARD.encode(keypair.verifying_key().as_bytes()),
            pub_expecteed
        );
        let got = general_purpose::STANDARD.encode(keypair.as_bytes());
        assert_eq!(got, expected);
    }
}
