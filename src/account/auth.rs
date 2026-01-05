//! Authentication and authorization for accounts

use serde::{Deserialize, Serialize};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use sha2::Sha256;

#[derive(Debug, Clone)]
pub enum AuthError {
    InvalidPassword,
    EncryptionFailed,
    DecryptionFailed,
    InvalidKey,
}

/// Authorization model for account signing
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AuthorizationModel {
    /// Single key authorization
    SingleKey { pubkey: String },
    
    /// Multi-signature (future use)
    MultiSig { threshold: u8, keys: Vec<String> },
    
    /// Key rotation support
    KeyRotation {
        current: String,
        scheduled: Option<String>,
        revoked: Vec<String>,
    },
}

impl AuthorizationModel {
    /// Check if a public key can authorize transactions
    pub fn can_sign(&self, pubkey: &str) -> bool {
        match self {
            Self::SingleKey { pubkey: pk } => pk == pubkey,
            Self::MultiSig { keys, .. } => keys.contains(&pubkey.to_string()),
            Self::KeyRotation { current, scheduled, .. } => {
                current == pubkey || scheduled.as_ref() == Some(&pubkey.to_string())
            }
        }
    }
}

/// Hash a password using Argon2id
pub fn hash_password(password: &str) -> Result<(String, Vec<u8>), AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| AuthError::InvalidPassword)?
        .to_string();
    
    // Store salt as string bytes (salt.as_str() gives the base64 string)
    Ok((password_hash, salt.as_str().as_bytes().to_vec()))
}

/// Verify a password against a hash
pub fn verify_password(password: &str, password_hash: &str) -> Result<(), AuthError> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|_| AuthError::InvalidPassword)?;
    
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AuthError::InvalidPassword)
}

/// Derive an encryption key from a password and salt
pub fn derive_encryption_key(password: &str, salt: &[u8]) -> Vec<u8> {
    use pbkdf2::pbkdf2;
    use hmac::Hmac;
    
    let mut key = [0u8; 32]; // 256-bit key
    pbkdf2::<Hmac<Sha256>>(password.as_bytes(), salt, 100_000, &mut key);
    key.to_vec()
}

/// Encrypt data using AES-256-GCM
pub fn encrypt_data(data: &[u8], password: &str, salt: &[u8]) -> Result<Vec<u8>, AuthError> {
    let key = derive_encryption_key(password, salt);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| AuthError::InvalidKey)?;
    
    // Generate random nonce
    let nonce_bytes: [u8; 12] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|_| AuthError::EncryptionFailed)?;
    
    // Prepend nonce to ciphertext
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    
    Ok(result)
}

/// Decrypt data using AES-256-GCM
pub fn decrypt_data(encrypted: &[u8], password: &str, salt: &[u8]) -> Result<Vec<u8>, AuthError> {
    if encrypted.len() < 12 {
        return Err(AuthError::DecryptionFailed);
    }
    
    let key = derive_encryption_key(password, salt);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| AuthError::InvalidKey)?;
    
    // Extract nonce and ciphertext
    let nonce = Nonce::from_slice(&encrypted[..12]);
    let ciphertext = &encrypted[12..];
    
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| AuthError::DecryptionFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_password_hashing() {
        let password = "my_secure_password_123";
        let (hash, _salt) = hash_password(password).unwrap();
        
        // Verify correct password
        assert!(verify_password(password, &hash).is_ok());
        
        // Verify wrong password
        assert!(verify_password("wrong_password", &hash).is_err());
    }
    
    #[test]
    fn test_encryption() {
        let data = b"sensitive data";
        let password = "encryption_password";
        let salt = b"random_salt_1234";
        
        let encrypted = encrypt_data(data, password, salt).unwrap();
        let decrypted = decrypt_data(&encrypted, password, salt).unwrap();
        
        assert_eq!(data, decrypted.as_slice());
    }
    
    #[test]
    fn test_authorization_model() {
        let auth = AuthorizationModel::SingleKey {
            pubkey: "0x1234".to_string(),
        };
        
        assert!(auth.can_sign("0x1234"));
        assert!(!auth.can_sign("0x5678"));
    }
}
