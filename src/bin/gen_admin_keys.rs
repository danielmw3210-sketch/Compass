use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
struct AdminKeyPair {
    public_key: String,
    private_key: String,
}

fn main() {
    println!("=== Compass Admin Key Generator ===\n");
    
    // Generate new ed25519 keypair
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    
    // Convert to hex
    let private_hex = hex::encode(signing_key.to_bytes());
    let public_hex = hex::encode(verifying_key.to_bytes());
    
    println!("Generated Admin Keypair:");
    println!("Public Key:  {}", public_hex);
    println!("Private Key: {} (KEEP SECRET!)\n", private_hex);
    
    // Save to admin_key.json
    let keypair = AdminKeyPair {
        public_key: public_hex.clone(),
        private_key: private_hex,
    };
    
    let json = serde_json::to_string_pretty(&keypair).unwrap();
    fs::write("admin_key.json", json).expect("Failed to write admin_key.json");
    println!("✓ Saved to admin_key.json");
    
    // Save public key separately for genesis config
    fs::write("admin_pubkey.txt", public_hex.clone()).expect("Failed to write admin_pubkey.txt");
    println!("✓ Saved public key to admin_pubkey.txt");
    
    println!("\n⚠️  IMPORTANT:");
    println!("1. Keep admin_key.json secure (this is your validator identity)");
    println!("2. Add the public key to genesis.json validators list:");
    println!("   {{");
    println!("     \"id\": \"Admin\",");
    println!("     \"public_key\": \"{}\",", public_hex);
    println!("     \"stake\": 1000000");
    println!("   }}");
}
