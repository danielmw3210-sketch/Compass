use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use hex;

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