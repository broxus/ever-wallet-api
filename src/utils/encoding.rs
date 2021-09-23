use chacha20poly1305::aead::{AeadMut, Result};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};

pub fn encrypt_private_key(private_key: &[u8], key: [u8; 32], id: &uuid::Uuid) -> Result<String> {
    use chacha20poly1305::aead::NewAead;
    let nonce = Nonce::from_slice(&id.as_bytes()[0..12]);
    let key = chacha20poly1305::Key::from_slice(&key[..]);
    let mut encryptor = ChaCha20Poly1305::new(key);
    let res = encryptor.encrypt(nonce, private_key)?;
    Ok(base64::encode(res))
}

pub fn decrypt_private_key(private_key: &str, key: [u8; 32], id: &uuid::Uuid) -> Result<Vec<u8>> {
    use chacha20poly1305::aead::NewAead;
    let nonce = Nonce::from_slice(&id.as_bytes()[0..12]);
    let key = chacha20poly1305::Key::from_slice(&key[..]);
    let mut decrypter = ChaCha20Poly1305::new(key);
    decrypter.decrypt(
        nonce,
        base64::decode(private_key).unwrap_or_default().as_slice(),
    )
}
