pub mod node;
pub mod ops;
pub mod tx;
pub mod wallet;
pub mod keys; // New Key Manager
pub mod session; // RBAC Session Management

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "compass")]
#[command(about = "Compass Blockchain CLI", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Key management (Identity)
    Keys {
        #[command(subcommand)]
        cmd: keys::KeysCommands,
    },
    /// Wallet management
    Wallet {
        #[command(subcommand)]
        cmd: wallet::WalletCommands,
    },
    /// Account management
    Account {
        #[command(subcommand)]
        cmd: wallet::AccountCommands,
    },
    /// Node operations
    Node {
        #[command(subcommand)]
        cmd: node::NodeCommands,
    },
    /// Transaction operations
    Transfer {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: u64,
        #[arg(long)]
        asset: String,
    },

    /// Run as AI Worker
    Worker {
        #[arg(long, default_value = "http://127.0.0.1:9000")]
        node_url: String,
        
        #[arg(long, default_value = "gpt-4o-mini")]
        model_id: String,
        
        #[arg(long, default_value = "worker")]
        wallet: String,
    },
    
    /// Interactive Mode (Default)
    Interactive,
    Balance {
        address: String,
    },
    /// Mint new Compass tokens via Vault
    Mint {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        amount: u64,
        #[arg(long)]
        asset: String, // Compass Asset to mint
        #[arg(long)]
        collateral_asset: String,
        #[arg(long)]
        collateral_amount: u64,
        #[arg(long)]
        proof: String, // External transaction hash
        #[arg(long)]
        oracle_sig: String, // Simulation for now
        #[arg(long)]
        owner: String, // Wallet Name
    },
    /// Burn Compass tokens to release collateral
    Burn {
        #[arg(long)]
        vault_id: String,
        #[arg(long)]
        amount: u64,
        #[arg(long)]
        asset: String, // Asset to burn
        #[arg(long)]
        dest_addr: String, // External address
        #[arg(long)]
        #[arg(long)]
        from: String, // Wallet Name (Redeemer)
    },

    /// Interactive Client Mode
    Client,
    /// Generate Admin Key and Genesis Config (Trusted Setup)
    AdminGen,
    /// Calculate Genesis Hash (Offline)
    GenesisHash,
    
    /// List an NFT for sale
    ListNFT {
        #[arg(long)]
        token_id: String,
        #[arg(long)]
        price: u64,
        #[arg(long, default_value = "Compass")]
        currency: String, // "Compass" or "SOL", etc.
        #[arg(long, default_value = "worker")]
        wallet: String,
    },
    
    /// Buy an NFT from the Marketplace
    BuyNFT {
        #[arg(long)]
        token_id: String,
        #[arg(long, default_value = "worker")]
        wallet: String,
    },
    
    /// Trigger training for all signal models (BTC, ETH, LTC, SOL)
    TrainModels,
}
