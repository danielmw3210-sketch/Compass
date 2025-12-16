use rust_compass::identity::{Identity, NodeRole};
use std::path::Path;

fn main() {
    println!("Generating new Admin Identity...");
    
    let password = "password123";
    let (identity, mnemonic) = Identity::new("admin", NodeRole::Admin, password).expect("Failed to create identity");
    
    println!("Identity Created!");
    println!("Public Key: {}", identity.public_key);
    println!("Mnemonic: {}", mnemonic);
    println!("Password: {}", password);
    
    let path = Path::new("dist/admin.json");
    identity.save(path).expect("Failed to save admin.json");
    
    println!("Saved to {:?}", path);
}
