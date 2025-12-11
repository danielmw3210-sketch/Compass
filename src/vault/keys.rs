use std::fs;
use std::path::Path;

/// Manages the master seed for vault address derivation
pub struct VaultKeyManager {
    master_seed: [u8; 64],
}

impl VaultKeyManager {
    /// Load existing seed or generate new one from mnemonic
    pub fn load_or_generate(path: &str) -> Self {
        if Path::new(path).exists() {
            // Load existing seed
            let seed_hex = fs::read_to_string(path)
                .expect("Failed to read master seed");
            let seed_bytes = hex::decode(seed_hex.trim())
                .expect("Invalid seed format");
            
            let mut master_seed = [0u8; 64];
            master_seed.copy_from_slice(&seed_bytes);
            
            println!("✓ Loaded vault master seed from {}", path);
            
            Self { master_seed }
        } else {
            // Generate new seed from mnemonic
            use rand::rngs::OsRng;
            let mut entropy = [0u8; 32];
            rand::RngCore::fill_bytes(&mut OsRng, &mut entropy);
            
            let mnemonic = bip39::Mnemonic::from_entropy(&entropy)
                .expect("Failed to generate mnemonic");
            let seed = mnemonic.to_seed("");
            
            // Save mnemonic for backup
            let mnemonic_path = format!("{}.mnemonic", path);
            fs::write(&mnemonic_path, mnemonic.to_string())
                .expect("Failed to save mnemonic");
            
            // Save seed
            fs::write(path, hex::encode(&seed))
                .expect("Failed to save master seed");
            
            println!("⚠️  NEW VAULT MASTER SEED GENERATED");
            println!("⚠️  Mnemonic backup saved to: {}", mnemonic_path);
            println!("⚠️  KEEP THIS SAFE - Required to recover vault addresses!");
            
            Self { master_seed: seed }
        }
    }
    
    pub fn get_seed(&self) -> &[u8; 64] {
        &self.master_seed
    }
}
