use clap::Subcommand;
use crate::identity::{Identity, NodeRole};
use std::path::Path;
use std::io::{self, Write};

#[derive(Subcommand, Debug, Clone)]
pub enum KeysCommands {
    /// Generate a new secure identity
    Generate {
        /// Role of the identity (admin, verifier, user)
        #[clap(long)]
        role: String,
        
        /// Name of the key file (e.g. "admin_key")
        #[clap(long)]
        name: String,
    },
    /// Export the Public Key to a file
    ExportPub {
         #[clap(long)]
        name: String,
    },
    /// Inspect a key file (verify password and integrity)
    Inspect {
        #[clap(long)]
        name: String,
    }
}

pub fn handle_keys_command(cmd: KeysCommands) {
    match cmd {
        KeysCommands::Generate { role, name } => {
            let role_enum = match role.parse::<NodeRole>() {
                Ok(r) => r,
                Err(e) => {
                    println!("Error: {}", e);
                    return;
                }
            };

            let filename = format!("{}.json", name);
            if Path::new(&filename).exists() {
                println!("Error: File '{}' already exists. Aborting to prevent overwrite.", filename);
                return;
            }

            println!("Creating new {} identity: '{}'", role, name);
            print!("Enter encryption password: ");
            io::stdout().flush().unwrap();
            
            let mut password = String::new();
            io::stdin().read_line(&mut password).unwrap();
            let password = password.trim();

            if password.len() < 4 {
                 println!("Error: Password too short (min 4 chars)");
                 return;
            }

            match Identity::new(&name, role_enum, password) {
                Ok((identity, mnemonic)) => {
                    if let Err(e) = identity.save(Path::new(&filename)) {
                        println!("Error saving file: {}", e);
                        return;
                    }
                    println!("\nSUCCESS: Identity saved to '{}'", filename);
                    println!("Public Key: {}", identity.public_key);
                    println!("\n[SECRET MNEMONIC] - Write this down securely and NEVER share it:");
                    println!("---------------------------------------------------------------");
                    println!("{}", mnemonic);
                    println!("---------------------------------------------------------------");
                },
                Err(e) => println!("Error generating identity: {}", e),
            }
        },
        KeysCommands::ExportPub { name } => {
             let filename = format!("{}.json", name);
             // We don't strictly need password just to read the public key struct field, 
             // but `load_and_decrypt` requires it. 
             // For safety, let's ask for password to verify ownership.
             print!("Enter password for '{}': ", filename);
             io::stdout().flush().unwrap();
             let mut pass = String::new();
             io::stdin().read_line(&mut pass).unwrap();
             
             match Identity::load_and_decrypt(Path::new(&filename), pass.trim()) {
                 Ok(id) => {
                     let pub_file = format!("{}_pub.txt", name);
                     std::fs::write(&pub_file, &id.public_key).unwrap();
                     println!("Public Key: {}", id.public_key);
                     println!("Exported to: {}", pub_file);
                 },
                 Err(e) => println!("Error loading key: {}", e),
             }
        },
        KeysCommands::Inspect { name } => {
             let filename = format!("{}.json", name);
             print!("Enter password: ");
             io::stdout().flush().unwrap();
             let mut pass = String::new();
             io::stdin().read_line(&mut pass).unwrap();
             
             match Identity::load_and_decrypt(Path::new(&filename), pass.trim()) {
                 Ok(id) => {
                     println!("\nIdentity Verified Integrity OK.");
                     println!("Name: {}", id.name);
                     println!("Role: {}", id.role);
                     println!("PubKey: {}", id.public_key);
                 },
                 Err(e) => println!("Error: {}", e),
             }
        }
    }
}
