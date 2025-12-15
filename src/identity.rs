#![allow(dead_code)]
use crate::crypto::KeyPair;
use ed25519_dalek::{SigningKey, Signer, Signature};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use rand::rngs::OsRng;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use pbkdf2::pbkdf2;
use hmac::Hmac;
use sha2::Sha256;
use hex;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Copy)]
pub enum NodeRole {
    Admin,
    Verifier,
    User,
}

impl std::fmt::Display for NodeRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeRole::Admin => write!(f, "admin"),
            NodeRole::Verifier => write!(f, "verifier"),
            NodeRole::User => write!(f, "user"),
        }
    }
}

impl std::str::FromStr for NodeRole {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(NodeRole::Admin),
            "verifier" => Ok(NodeRole::Verifier),
            "user" => Ok(NodeRole::User),
            _ => Err(format!("Invalid role: {}. Allowed: admin, verifier, user", s)),
        }
    }
}

/// A strictly typed Identity for a Node
#[derive(Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub role: NodeRole,
    pub public_key: String, // Hex encoded
    #[serde(skip_serializing, skip_deserializing)]
    inner_key: Option<SigningKey>, // Loaded in memory only
    encrypted_mnemonic: Vec<u8>,
    encryption_salt: Vec<u8>,
}

impl Identity {
    /// Create a new Identity (Generates fresh keys)
    pub fn new(name: &str, role: NodeRole, password: &str) -> Result<(Self, String), String> {
        // 1. Generate Mnemonic
        let mnemonic = KeyPair::generate_mnemonic();
        let keypair = KeyPair::from_mnemonic(&mnemonic).map_err(|e| e)?;
        
        let pubkey_hex = keypair.public_key_hex();
        
        // 2. Encrypt Mnemonic immediately
        let (encrypted, salt) = Self::encrypt_mnemonic(&mnemonic, password)?;

        let identity = Identity {
            name: name.to_string(),
            role,
            public_key: pubkey_hex,
            inner_key: Some(keypair.signing_key),
            encrypted_mnemonic: encrypted,
            encryption_salt: salt,
        };

        Ok((identity, mnemonic))
    }

    /// Create Identity from existing Mnemonic (for recovery or wallet loading)
    pub fn from_mnemonic(name: &str, role: NodeRole, mnemonic: &str, password: &str) -> Result<(Self, String), String> {
        let keypair = KeyPair::from_mnemonic(mnemonic).map_err(|e| e)?;
        let pubkey_hex = keypair.public_key_hex();
        
        let (encrypted, salt) = Self::encrypt_mnemonic(mnemonic, password)?;

        let identity = Identity {
            name: name.to_string(),
            role,
            public_key: pubkey_hex,
            inner_key: Some(keypair.signing_key),
            encrypted_mnemonic: encrypted,
            encryption_salt: salt,
        };

        Ok((identity, mnemonic.to_string()))
    }

    fn encrypt_mnemonic(mnemonic: &str, password: &str) -> Result<(Vec<u8>, Vec<u8>), String> {
        let mut salt = [0u8; 16];
        use rand::RngCore;
        OsRng.fill_bytes(&mut salt);

        let mut key = [0u8; 32];
        pbkdf2::<Hmac<Sha256>>(password.as_bytes(), &salt, 100_000, &mut key);

        let cipher = Aes256Gcm::new(&key.into());
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, mnemonic.as_bytes())
            .map_err(|e| format!("Encryption error: {:?}", e))?;

        let mut blob = Vec::new();
        blob.extend_from_slice(&nonce_bytes);
        blob.extend_from_slice(&ciphertext);

        Ok((blob, salt.to_vec()))
    }

    pub fn load_and_decrypt(path: &Path, password: &str) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut identity: Identity = serde_json::from_str(&content).map_err(|e| e.to_string())?;

        // Decrypt
        if identity.encrypted_mnemonic.len() < 12 {
            return Err("Invalid encrypted data file".to_string());
        }

        let nonce_bytes = &identity.encrypted_mnemonic[0..12];
        let ciphertext = &identity.encrypted_mnemonic[12..];
        
        let mut key = [0u8; 32];
        pbkdf2::<Hmac<Sha256>>(password.as_bytes(), &identity.encryption_salt, 100_000, &mut key);

        let cipher = Aes256Gcm::new(&key.into());
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|_| "Wrong password or corrupted file".to_string())?;

        let mnemonic = String::from_utf8(plaintext).map_err(|_| "Invalid UTF8".to_string())?;
        
        // Restore Inner Key
        let keypair = KeyPair::from_mnemonic(&mnemonic).map_err(|e| e)?;
        
        // Verify Integrity
        if keypair.public_key_hex() != identity.public_key {
            return Err("CRITICAL: Decrypted key does not match stored public key! File corrupted or tampered.".to_string());
        }

        identity.inner_key = Some(keypair.signing_key);
        Ok(identity)
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        // We only save the serializable parts (encrypted blob, no inner key)
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Sign a message (MUST be unlocked)
    pub fn sign(&self, message: &[u8]) -> Result<Signature, String> {
        let k = self.inner_key.as_ref().ok_or("Identity is locked/encrypted")?;
        Ok(k.sign(message))
    }
    
    /// Sign and return Hex (Convenience)
    pub fn sign_hex(&self, message: &[u8]) -> Result<String, String> {
        let sig = self.sign(message)?;
        Ok(hex::encode(sig.to_bytes()))
    }

    /// Convert to Crypto KeyPair (consumed)
    pub fn into_keypair(self) -> Result<KeyPair, String> {
        let sk = self.inner_key.ok_or("Identity locked")?;
        Ok(KeyPair { signing_key: sk })
    }
}
