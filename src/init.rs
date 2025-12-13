#![allow(dead_code)]
/// First-time initialization module
/// Automatically generates admin keys, verifier keys, and worker wallets

use crate::crypto::KeyPair;
use std::fs;
use std::path::Path;

pub struct InitializedKeys {
    pub admin_keypair: KeyPair,
    pub admin_address: String,
    pub admin_mnemonic: String,
    pub verifier_keypair: Option<KeyPair>,
    pub verifier_address: Option<String>,
    pub verifier_mnemonic: Option<String>,
}

/// Initialize or load admin and verifier keys
pub fn initialize_keys(generate_verifier: bool) -> InitializedKeys {
    println!("\n🔐 === Key Initialization ===");
    
    // 1. Admin Keys
    let (admin_keypair, admin_address, admin_mnemonic) = load_or_generate_admin();
    
    // 2. Verifier Keys (optional)
    let (verifier_keypair, verifier_address, verifier_mnemonic) = if generate_verifier {
        let (kp, addr, mnem) = load_or_generate_verifier();
        (Some(kp), Some(addr), Some(mnem))
    } else {
        (None, None, None)
    };
    
    println!("✅ Key initialization complete!\n");
    
    InitializedKeys {
        admin_keypair,
        admin_address,
        admin_mnemonic,
        verifier_keypair,
        verifier_address,
        verifier_mnemonic,
    }
}

/// Load or generate admin keys
fn load_or_generate_admin() -> (KeyPair, String, String) {
    let admin_file = "admin_keys.json";
    
    // Try to load existing
    if Path::new(admin_file).exists() {
        if let Ok(json_data) = fs::read_to_string(admin_file) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json_data) {
                if let Some(mnemonic) = data.get("mnemonic").and_then(|v| v.as_str()) {
                    if let Ok(keypair) = KeyPair::from_mnemonic(mnemonic) {
                        let address = keypair.public_key_hex();
                        println!("✅ Loaded existing admin keys from {}", admin_file);
                        println!("   Admin Address: {}", address);
                        return (keypair, address, mnemonic.to_string());
                    }
                }
            }
        }
    }
    
    // Generate new admin keys
    println!("🆕 Generating new admin keys...");
    let mnemonic = KeyPair::generate_mnemonic();
    let keypair = KeyPair::from_mnemonic(&mnemonic)
        .expect("Failed to create admin keypair");
    let address = keypair.public_key_hex();
    
    // Save to disk
    let admin_data = serde_json::json!({
        "role": "admin",
        "mnemonic": mnemonic,
        "address": address,
        "created_at": chrono::Utc::now().to_rfc3339(),
    });
    
    if let Ok(json_str) = serde_json::to_string_pretty(&admin_data) {
        let _ = fs::write(admin_file, json_str);
        println!("💾 Admin keys saved to {}", admin_file);
    }
    
    println!("\n⚠️  CRITICAL: Save your admin mnemonic securely!");
    println!("📝 Admin Mnemonic: {}", mnemonic);
    println!("🔑 Admin Address: {}\n", address);
    
    (keypair, address, mnemonic)
}

/// Load or generate verifier keys
fn load_or_generate_verifier() -> (KeyPair, String, String) {
    let verifier_file = "verifier_keys.json";
    
    // Try to load existing
    if Path::new(verifier_file).exists() {
        if let Ok(json_data) = fs::read_to_string(verifier_file) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json_data) {
                if let Some(mnemonic) = data.get("mnemonic").and_then(|v| v.as_str()) {
                    if let Ok(keypair) = KeyPair::from_mnemonic(mnemonic) {
                        let address = keypair.public_key_hex();
                        println!("✅ Loaded existing verifier keys from {}", verifier_file);
                        println!("   Verifier Address: {}", address);
                        return (keypair, address, mnemonic.to_string());
                    }
                }
            }
        }
    }
    
    // Generate new verifier keys
    println!("🆕 Generating new verifier keys...");
    let mnemonic = KeyPair::generate_mnemonic();
    let keypair = KeyPair::from_mnemonic(&mnemonic)
        .expect("Failed to create verifier keypair");
    let address = keypair.public_key_hex();
    
    // Save to disk
    let verifier_data = serde_json::json!({
        "role": "verifier",
        "mnemonic": mnemonic,
        "address": address,
        "created_at": chrono::Utc::now().to_rfc3339(),
    });
    
    if let Ok(json_str) = serde_json::to_string_pretty(&verifier_data) {
        let _ = fs::write(verifier_file, json_str);
        println!("💾 Verifier keys saved to {}", verifier_file);
    }
    
    println!("\n⚠️  CRITICAL: Save your verifier mnemonic securely!");
    println!("📝 Verifier Mnemonic: {}", mnemonic);
    println!("🔑 Verifier Address: {}\n", address);
    
    (keypair, address, mnemonic)
}

/// Generate worker wallet (for oracle verification workers)
pub fn generate_worker_wallet(worker_id: &str) -> (KeyPair, String, String) {
    let wallet_file = format!("worker_{}_keys.json", worker_id);
    
    // Try to load existing
    if Path::new(&wallet_file).exists() {
        if let Ok(json_data) = fs::read_to_string(&wallet_file) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json_data) {
                if let Some(mnemonic) = data.get("mnemonic").and_then(|v| v.as_str()) {
                    if let Ok(keypair) = KeyPair::from_mnemonic(mnemonic) {
                        let address = keypair.public_key_hex();
                        println!("✅ Loaded existing worker wallet: {}", address);
                        return (keypair, address, mnemonic.to_string());
                    }
                }
            }
        }
    }
    
    // Generate new worker wallet
    println!("🆕 Generating new worker wallet for {}...", worker_id);
    let mnemonic = KeyPair::generate_mnemonic();
    let keypair = KeyPair::from_mnemonic(&mnemonic)
        .expect("Failed to create worker keypair");
    let address = keypair.public_key_hex();
    
    // Save to disk
    let worker_data = serde_json::json!({
        "role": "worker",
        "worker_id": worker_id,
        "mnemonic": mnemonic,
        "address": address,
        "created_at": chrono::Utc::now().to_rfc3339(),
    });
    
    if let Ok(json_str) = serde_json::to_string_pretty(&worker_data) {
        let _ = fs::write(&wallet_file, json_str);
        println!("💾 Worker wallet saved to {}", wallet_file);
    }
    
    println!("🔑 Worker Address: {}", address);
    println!("📝 Worker Mnemonic: {}\n", mnemonic);
    
    (keypair, address, mnemonic)
}
