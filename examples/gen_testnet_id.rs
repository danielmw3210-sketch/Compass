use rust_compass::identity::{Identity, NodeRole};
use std::path::Path;

fn main() {
    let filename = "testnet_identity.json";
    let password = ""; // PASSWORDLESS for automation
    
    if Path::new(filename).exists() {
        println!("Identity file '{}' already exists. Skipping generation.", filename);
        return;
    }
    
    println!("Generating passwordless identity for Testnet...");
    match Identity::new("testnet_admin", NodeRole::Admin, password) {
        Ok((identity, _mnemonic)) => {
            if let Err(e) = identity.save(Path::new(filename)) {
                eprintln!("Error saving identity: {}", e);
                std::process::exit(1);
            }
            println!("SUCCESS: Identity saved to '{}'", filename);
            println!("Public Key: {}", identity.public_key);
        },
        Err(e) => {
            eprintln!("Error generating identity: {}", e);
            std::process::exit(1);
        },
    }
}
