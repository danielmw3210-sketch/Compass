pub mod node;
pub mod ops;
pub mod tx;
pub mod wallet;

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
    /// Start AI Worker
    Worker {
        #[arg(long, default_value = "http://localhost:8899")]
        node_url: String,
        #[arg(long, default_value = "llama-2-7b-q4")]
        model_id: String,
    },
}
