use crate::crypto::KeyPair;
use crate::wallet::{Wallet, WalletManager, WalletType};
use clap::Subcommand;

#[derive(Subcommand)]
pub enum WalletCommands {
    /// Create a new wallet
    Create {
        #[arg(long)]
        name: String,
    },
    /// Import a wallet from mnemonic
    Import {
        #[arg(long)]
        mnemonic: String,
        #[arg(long)]
        name: String,
    },
    /// List all wallets
    List,
}

#[derive(Subcommand)]
pub enum AccountCommands {
    /// Create an account (derived from wallet) - Simplified for now
    Create {
        #[arg(long)]
        wallet: String,
    },
    List,
    ExportPubkey {
        #[arg(long)]
        wallet: String,
    },
}

pub fn handle_wallet_command(cmd: WalletCommands) {
    // For now, load/save from local file "wallets.json" in current dir
    // This is distinct from the Node's wallet manager, but sharing struct for now.
    let mut manager = WalletManager::load("wallets.json");

    match cmd {
        WalletCommands::Create { name } => {
            let wallet = Wallet::new(&name, WalletType::User);
            if let Some(mnemonic) = &wallet.mnemonic {
                println!("Wallet '{}' created.", name);
                println!("Mnemonic: {}", mnemonic);
                println!("Public Key: {}", wallet.public_key);
                println!("KEEP THIS SAFE!");
            }
            manager.wallets.insert(wallet.owner.clone(), wallet);
            let _ = manager.save("wallets.json");
        }
        WalletCommands::Import { mnemonic, name } => {
            // Validate mnemonic
            match KeyPair::from_mnemonic(&mnemonic) {
                Ok(kp) => {
                    let mut wallet = Wallet::new(&name, WalletType::User);
                    // Overwrite with imported keys
                    wallet.mnemonic = Some(mnemonic);
                    wallet.public_key = kp.public_key_hex();
                    manager.wallets.insert(wallet.owner.clone(), wallet);
                    let _ = manager.save("wallets.json");
                    println!("Wallet '{}' imported successfully.", name);
                }
                Err(e) => println!("Failed to import: {}", e),
            }
        }
        WalletCommands::List => {
            for w in manager.wallets.values() {
                println!("Name: {}\tAddress: {}", w.owner, w.public_key);
            }
        }
    }
}

pub fn handle_account_command(cmd: AccountCommands) {
    let manager = WalletManager::load("wallets.json");
    match cmd {
        AccountCommands::Create { wallet: _ } => {
            println!("Account creation not fully implemented via derived paths yet.");
        }
        AccountCommands::List => {
            // Same as wallet list for now
            // Same as wallet list for now
            for w in manager.wallets.values() {
                println!("Account: {}\tPK: {}", w.owner, w.public_key);
            }
        }
        AccountCommands::ExportPubkey { wallet } => {
            if let Some(w) = manager.get_wallet(&wallet) {
                println!("{}", w.public_key);
            } else {
                println!("Wallet not found");
            }
        }
    }
}
