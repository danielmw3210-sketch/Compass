use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use rand::RngCore; // Imported for fill_bytes
use hex;
use bip39::{Mnemonic, Language};

pub struct KeyPair {
    pub keypair: Keypair,
}

impl KeyPair {
    /// Generate a new Ed25519 keypair
    pub fn new() -> Self {
        let mut csprng = OsRng;
        let keypair = Keypair::generate(&mut csprng);
        KeyPair { keypair }
    }
    
    /// Generate a new 12-word mnemonic
    pub fn generate_mnemonic() -> String {
        let mut entropy = [0u8; 16]; // 128 bits = 12 words
        let mut csprng = OsRng;
        csprng.fill_bytes(&mut entropy);
        let mnemonic = Mnemonic::from_entropy(&entropy).expect("Failed to create mnemonic");
        mnemonic.to_string()
    }

    /// Restore keypair from mnemonic
    pub fn from_mnemonic(phrase: &str) -> Result<Self, String> {
        let mnemonic = Mnemonic::parse_in_normalized(Language::English, phrase)
            .map_err(|e| format!("Invalid mnemonic: {}", e))?;
        let seed = mnemonic.to_seed("");
        
        // Use first 32 bytes for Ed25519 SecretKey
        let secret = ed25519_dalek::SecretKey::from_bytes(&seed[0..32])
            .map_err(|e| e.to_string())?;
        let public = ed25519_dalek::PublicKey::from(&secret);
        let keypair = ed25519_dalek::Keypair { secret, public };
        
        Ok(KeyPair { keypair })
    }

    /// Sign a message with the private key
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.keypair.sign(message)
    }

    /// Verify a signature against a message using this keypair's public key
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        self.keypair.public.verify(message, signature).is_ok()
    }

    /// Get the public key
    pub fn public_key(&self) -> PublicKey {
        self.keypair.public
    }

    /// Sign a message and return hex string
    pub fn sign_hex(&self, message: &[u8]) -> String {
        let signature = self.sign(message);
        hex::encode(signature.to_bytes())
    }

    /// Verify a hex signature string against a message using this keypair's public key
    pub fn verify_hex(&self, message: &[u8], signature_hex: &str) -> bool {
        if let Ok(bytes) = hex::decode(signature_hex) {
            if let Ok(signature) = Signature::from_bytes(&bytes) {
                return self.verify(message, &signature);
            }
        }
        false
    }

    /// Get public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key().to_bytes())
    }
}

/// Verify a signature against a message with a provided public key (hex)
pub fn verify_with_pubkey_hex(message: &[u8], signature_hex: &str, pubkey_hex: &str) -> bool {
    if let (Ok(sig_bytes), Ok(pk_bytes)) = (hex::decode(signature_hex), hex::decode(pubkey_hex)) {
        if let (Ok(signature), Ok(pubkey)) = (Signature::from_bytes(&sig_bytes), PublicKey::from_bytes(&pk_bytes)) {
            return pubkey.verify(message, &signature).is_ok();
        }
    }
    false
}
