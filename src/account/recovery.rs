//! Account recovery using BIP39 mnemonic seed phrases

use bip39::{Mnemonic, Language};
use tiny_hderive::bip32::ExtendedPrivKey;

#[derive(Debug, Clone)]
pub enum RecoveryError {
    InvalidMnemonic,
    KeyDerivationFailed,
}

/// Recovery key system using BIP39 mnemonic
pub struct RecoveryKey {
    pub mnemonic: Mnemonic,
}

impl RecoveryKey {
    /// Generate a new 24-word recovery key
    pub fn generate() -> Result<Self, RecoveryError> {
        use rand::RngCore;
        use rand::rngs::OsRng;
        
        // Generate 32 bytes of entropy (256 bits for 24 words)
        let mut entropy = [0u8; 32];
        OsRng.fill_bytes(&mut entropy);
        
        let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
            .map_err(|_| RecoveryError::InvalidMnemonic)?;
        
        Ok(Self { mnemonic })
    }
    
    /// Create from existing mnemonic phrase
    pub fn from_phrase(phrase: &str) -> Result<Self, RecoveryError> {
        let mnemonic = Mnemonic::parse_in(Language::English, phrase)
            .map_err(|_| RecoveryError::InvalidMnemonic)?;
        
        Ok(Self { mnemonic })
    }
    
    /// Get the mnemonic as a string (24 words)
    pub fn to_phrase(&self) -> String {
        self.mnemonic.words().collect::<Vec<&str>>().join(" ")
    }
    
    /// Derive a keypair from the mnemonic
    pub fn derive_keypair(&self, index: u32) -> Result<(Vec<u8>, Vec<u8>), RecoveryError> {
        let seed = self.mnemonic.to_seed("");
        
        // Derive using BIP32 path: m/44'/60'/0'/0/{index}
        // Using Ethereum's coin type (60) for now
        let path = format!("m/44'/60'/0'/0/{}", index);
        
        let ext_key = ExtendedPrivKey::derive(&seed, path.as_str())
            .map_err(|_| RecoveryError::KeyDerivationFailed)?;
        
        let privkey = ext_key.secret();
        
        // Derive public key from private key (using ed25519 or secp256k1)
        // For now, using simple derivation - will integrate with crypto module
        let pubkey = derive_pubkey_from_privkey(&privkey);
        
        Ok((privkey.to_vec(), pubkey))
    }
    
    /// Get the recovery public key (for verification)
    pub fn recovery_pubkey(&self) -> Result<Vec<u8>, RecoveryError> {
        let (_, pubkey) = self.derive_keypair(0)?;
        Ok(pubkey)
    }
}

/// Derive public key from private key
/// TODO: Integrate with existing crypto module
fn derive_pubkey_from_privkey(privkey: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    
    // Temporary simple derivation - will use proper curve cryptography
    let mut hasher = Sha256::new();
    hasher.update(privkey);
    hasher.update(b"_pubkey");
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_recovery_key_generation() {
        let recovery = RecoveryKey::generate().unwrap();
        let phrase = recovery.to_phrase();
        
        // Should be 24 words
        assert_eq!(phrase.split_whitespace().count(), 24);
        
        // Should be able to recreate from phrase
        let recovered = RecoveryKey::from_phrase(&phrase).unwrap();
        assert_eq!(recovery.to_phrase(), recovered.to_phrase());
    }
    
    #[test]
    fn test_keypair_derivation() {
        let recovery = RecoveryKey::generate().unwrap();
        let (privkey, pubkey) = recovery.derive_keypair(0).unwrap();
        
        assert_eq!(privkey.len(), 32);
        assert_eq!(pubkey.len(), 32);
        
        // Deriving same index should give same keys
        let (privkey2, pubkey2) = recovery.derive_keypair(0).unwrap();
        assert_eq!(privkey, privkey2);
        assert_eq!(pubkey, pubkey2);
        
        // Different index should give different keys
        let (privkey3, _) = recovery.derive_keypair(1).unwrap();
        assert_ne!(privkey, privkey3);
    }
}
