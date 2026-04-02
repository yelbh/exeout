use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::{RngCore, rngs::OsRng};
use anyhow::Result;

pub fn generate_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    key
}

pub fn encrypt_file(content: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(key.into());
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, content)
        .map_err(|e| anyhow::anyhow!("Encryption error: {}", e))?;

    let mut result = nonce_bytes.to_vec();
    result.extend(ciphertext);
    Ok(result)
}

pub fn decrypt_file(encrypted_content: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    if encrypted_content.len() < 12 {
        return Err(anyhow::anyhow!("Invalid ciphertext length"));
    }
    let (nonce_bytes, ciphertext) = encrypted_content.split_at(12);
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("Decryption error: {}", e))
}
