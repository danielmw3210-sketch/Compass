use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use bip39::{Language, Mnemonic};
use hex;
use rand::rngs::OsRng;
use rand::RngCore;

// Using VerifyingKey as PublicKey in API
pub type PublicKey = VerifyingKey;

pub struct KeyPair {
    pub signing_key: SigningKey,
}

impl KeyPair {
    /// Generate a new Ed25519 keypair
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        KeyPair { signing_key }
    }

    /// Generate a new 12-word mnemonic
    pub fn generate_mnemonic() -> String {
        let mut entropy = [0u8; 16]; // 128 bits = 12 words
        OsRng.fill_bytes(&mut entropy);
        Mnemonic::from_entropy(&entropy).unwrap().to_string()
    }

    /// Restore keypair from mnemonic phrase
    pub fn from_mnemonic(phrase: &str) -> Result<Self, String> {
        let mnemonic = Mnemonic::parse_in_normalized(Language::English, phrase)
            .map_err(|e| format!("Invalid mnemonic: {}", e))?;
        let seed = mnemonic.to_seed("");

        // Use first 32 bytes as secret key
        let secret_bytes: [u8; 32] = seed[0..32]
            .try_into()
            .map_err(|_| "Invalid seed length")?;
        
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        // Note: Dalek 2 from_bytes constructs SigningKey directly from 32 bytes.

        Ok(KeyPair { signing_key })
    }

    /// Sign a message with the private key
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }

    /// Sign and return hex-encoded signature
    pub fn sign_hex(&self, message: &[u8]) -> String {
        let sig = self.sign(message);
        hex::encode(sig.to_bytes())
    }
    
    /// Get the public key
    pub fn public_key(&self) -> PublicKey {
        self.signing_key.verifying_key()
    }

    /// Get public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.signing_key.verifying_key().to_bytes())
    }

    pub fn secret_key_hex(&self) -> String {
        hex::encode(self.signing_key.to_bytes())
    }

    pub fn from_secret_hex(hex_str: &str) -> Result<Self, String> {
        let bytes = hex::decode(hex_str).map_err(|e| e.to_string())?;
        if bytes.len() != 32 {
            return Err("Invalid secret key length".to_string());
        }
        let arr: [u8; 32] = bytes.try_into().map_err(|_| "Invalid length".to_string())?;
        let signing_key = SigningKey::from_bytes(&arr);
        Ok(KeyPair { signing_key })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != 32 {
            return Err("Invalid secret key bytes length (expected 32)".to_string());
        }
        let arr: [u8; 32] = bytes.try_into().map_err(|_| "Invalid bytes".to_string())?;
        let signing_key = SigningKey::from_bytes(&arr);
        Ok(KeyPair { signing_key })
    }

    /// Verify a signature against a message using this keypair's public key
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        self.signing_key.verify(message, signature).is_ok()
    }

    /// Verify a hex-encoded signature
    pub fn verify_hex(&self, message: &[u8], signature_hex: &str) -> bool {
        if let Ok(sig_bytes) = hex::decode(signature_hex) {
            if sig_bytes.len() == 64 {
                if let Ok(sig_array) = TryInto::<[u8; 64]>::try_into(sig_bytes) {
                    if let Ok(signature) = Signature::try_from(&sig_array[..]) {
                        return self.verify(message, &signature);
                    }
                }
            }
        }
        false
    }
}

/// Verify a signature with a public key (both hex-encoded)
pub fn verify_with_pubkey_hex(message: &[u8], signature_hex: &str, pubkey_hex: &str) -> bool {
    // Decode public key
    let pubkey_bytes = match hex::decode(pubkey_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    
    if pubkey_bytes.len() != 32 {
        return false;
    }
    
    let pub_array: [u8; 32] = match pubkey_bytes.as_slice().try_into() {
        Ok(arr) => arr,
        Err(_) => return false,
    };
    
    let pubkey = match VerifyingKey::from_bytes(&pub_array) {
        Ok(pk) => pk,
        Err(_) => return false,
    };

    // Decode signature
    let sig_bytes = match hex::decode(signature_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    
    if sig_bytes.len() != 64 {
        return false;
    }
    
    let sig_array: [u8; 64] = match sig_bytes.as_slice().try_into() {
        Ok(arr) => arr,
        Err(_) => return false,
    };
    
    let signature = match Signature::try_from(&sig_array[..]) {
        Ok(sig) => sig,
        Err(_) => return false,
    };

    // Verify
    pubkey.verify(message, &signature).is_ok()
}

/// Verify a signature with a public key (signature as bytes, pubkey as hex)
pub fn verify_with_pubkey_hex_bytes(
    message: &[u8],
    signature_bytes: &[u8],
    pubkey_hex: &str,
) -> Result<bool, String> {
    // Decode public key
    let pubkey_bytes = hex::decode(pubkey_hex).map_err(|_| "Invalid pubkey hex")?;
    let pub_array: [u8; 32] = pubkey_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid pubkey length")?;
    let verifying_key = VerifyingKey::from_bytes(&pub_array).map_err(|_| "Invalid pubkey bytes")?;

    // Parse signature
    if signature_bytes.len() != 64 {
        return Err("Invalid signature length".to_string());
    }
    let sig_array: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| "Invalid signature")?;
    let signature = Signature::try_from(&sig_array[..]).map_err(|_| "Invalid signature format")?;

    // Verify
    match verifying_key.verify(message, &signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
