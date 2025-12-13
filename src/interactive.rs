use std::io::{self, Write};
use crate::identity::{Identity, NodeRole};
use crate::cli::{self, keys::KeysCommands};

use std::path::Path;
use std::sync::Arc;
use crate::crypto::KeyPair;

pub async fn start() {
    print_banner();
    println!("1. Start Admin Node (Leader)");
    println!("2. Start Verifier Node (Worker)");
    println!("3. Start User Client (Wallet)");
    println!("4. Key Management (Generate/Export)");
    println!("5. Tools (Wipe DB, Network Status)");
    println!("6. Start AI Worker (Layer 3 Compute)");
    println!("7. Start Oracle Verification Worker (Earn COMPASS)");
    println!("9. Neural Networks: Buy & Train (User Mode)");
    println!("8. Exit");
    print!("\nSelect Option: ");
    io::stdout().flush().unwrap();

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();
    
    match choice.trim() {
        "1" => run_admin_node().await,
        "2" => run_verifier_node().await,
        "3" => crate::run_client_mode().await,
        "4" => key_menu(),
        "5" => tools_menu().await,
        "6" => run_ai_worker().await,
        "7" => run_oracle_verification_worker().await,
        "9" => crate::layer3::user_ops::run_user_ai_menu().await,
        "8" => std::process::exit(0),
        _ => {
            println!("Invalid option.");
            std::process::exit(1);
        },
    }
    
    // Enforce "One Point Entry" - Program ends after the selected mode finishes.
    println!("\nSession ended. Exiting.");
    std::process::exit(0);
}

async fn init_layer3_job() {
    println!("\n🚀 Initializing Layer 3 Finance Oracle...");
    println!("   Target: TCP 9000 (Localhost)");
    let client = crate::client::RpcClient::new("http://127.0.0.1:9000".to_string());
    match crate::layer3::finance_oracle::init_finance_oracle_job(&client).await {
        Ok(_) => println!("✓ Initialization sequence complete."),
        Err(e) => println!("❌ Error: {}", e),
    }
}

fn print_banner() {
    print!("\x1B[2J\x1B[1;1H"); 
    println!("========================================");
    println!("         COMPASS BLOCKCHAIN OS          ");
    println!("        v0.2.1 - Layer 3 Enabled        ");
    println!("========================================");
}

async fn run_admin_node() {
    println!("\n--- Admin Node (Leader) ---");
    let identity = match load_identity("admin") {
        Some(id) => id,
        None => {
            println!("Admin Identity not found or Password Incorrect!");
            println!("❌ ACCESS DENIED. Booting out...");
            std::thread::sleep(std::time::Duration::from_secs(1));
            std::process::exit(1); // BOOTED OUT
        }
    };
    
    if identity.role != NodeRole::Admin {
        println!("Error: Selected identity is NOT an Admin role.");
        std::process::exit(1);
    }
    
    let keypair = identity.into_keypair().expect("Failed to unlock keypair");
    println!("Identity Unlocked. PubKey: {}", keypair.public_key_hex());

    println!("Starting Leader Node on Port 8090...");
    
    // --- Leader ---
    // Override Default Config
    let mut leader_config = crate::config::CompassConfig::default();
    leader_config.node.rpc_port = 8090;
    leader_config.node.db_path = "compass_db_leader".to_string();
    
    crate::run_node_mode_internal(
        leader_config,
        None,
        Some(Arc::new(keypair)),
    ).await;
}

async fn run_verifier_node() {
    println!("\n--- Verifier Node (Worker) ---");
    let identity = match load_identity("verifier") {
        Some(id) => id,
        None => {
            println!("Verifier Identity not found or Password Incorrect!");
            println!("❌ ACCESS DENIED. Booting out...");
            std::thread::sleep(std::time::Duration::from_secs(1));
            std::process::exit(1);
        }
    };
    
    if identity.role != NodeRole::Verifier {
        println!("Error: Selected identity is NOT a Verifier role.");
        std::process::exit(1);
    }
    
    let keypair = identity.into_keypair().expect("Failed to unlock keypair");

    println!("Starting Verifier Node on Port 8091...");
    
    let mut verifier_config = crate::config::CompassConfig::default();
    verifier_config.node.rpc_port = 8091;
    verifier_config.node.db_path = "compass_db_verifier".to_string();

    crate::run_node_mode_internal(
        verifier_config,
        Some("127.0.0.1:8090".to_string()),
        Some(Arc::new(keypair))
    ).await;
}

pub fn load_identity(name: &str) -> Option<Identity> {
    let filename = format!("{}.json", name);
    if !Path::new(&filename).exists() {
        return None;
    }
    
    print!("Enter password for '{}': ", name);
    io::stdout().flush().unwrap();
    let mut pass = String::new();
    io::stdin().read_line(&mut pass).unwrap();
    
    match Identity::load_and_decrypt(Path::new(&filename), pass.trim()) {
        Ok(id) => Some(id),
        Err(e) => {
            println!("Authentication Failed: {}", e);
            None
        }
    }
}


async fn run_oracle_verification_worker() {
    println!("\n🔍 Starting Oracle Verification Worker...");
    
    print!("Enter Node URL (default: http://localhost:9000): ");
    io::stdout().flush().unwrap();
    let mut node_url = String::new();
    io::stdin().read_line(&mut node_url).unwrap();
    let node_url = if node_url.trim().is_empty() {
        "http://localhost:9000".to_string()
    } else {
        node_url.trim().to_string()
    };
    
    if let Err(e) = crate::worker_menu::worker_job_menu(&node_url).await {
        println!("Worker error: {}", e);
        println!("\nPress Enter to continue...");
        let mut _pause = String::new();
        io::stdin().read_line(&mut _pause).unwrap();
    }
}

fn key_menu() {
    println!("\n--- Key Management ---");
    println!("1. Generate Admin Key");
    println!("2. Generate Verifier Key");
    println!("3. Generate User Key");
    println!("4. Export Public Key");
    println!("5. Back");
    print!("Select: ");
    io::stdout().flush().unwrap();
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();
    
    match choice.trim() {
        "1" => crate::cli::keys::handle_keys_command(KeysCommands::Generate { role: "admin".to_string(), name: "admin".to_string() }),
        "2" => crate::cli::keys::handle_keys_command(KeysCommands::Generate { role: "verifier".to_string(), name: "verifier".to_string() }),
        "3" => {
            print!("Enter user name: ");
            io::stdout().flush().unwrap();
            let mut name = String::new();
            io::stdin().read_line(&mut name).unwrap();
            crate::cli::keys::handle_keys_command(KeysCommands::Generate { role: "user".to_string(), name: name.trim().to_string() });
        },
        "4" => {
            print!("Enter identity name: ");
            io::stdout().flush().unwrap();
            let mut name = String::new();
            io::stdin().read_line(&mut name).unwrap();
            crate::cli::keys::handle_keys_command(KeysCommands::ExportPub { name: name.trim().to_string() });
        },
        _ => {},
    }
}

async fn tools_menu() {
    println!("\n--- Tools ---");
    println!("1. Wipe Leader DB");
    println!("2. Wipe Verifier DB");
    println!("3. Generate Admin Genesis Config");
    println!("4. Init Finance Oracle (Admin Only)");
    println!("5. Back");
    print!("Select: ");
    io::stdout().flush().unwrap();
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();

    match choice.trim() {
        "1" => {
            println!("Wiping 'compass_db_leader'...");
            let _ = std::fs::remove_dir_all("compass_db_leader");
            println!("Done.");
        },
        "2" => {
            println!("Wiping 'compass_db_verifier'...");
             let _ = std::fs::remove_dir_all("compass_db_verifier");
            println!("Done.");
        },
        "3" => crate::genesis::generate_admin_config(),
        "4" => init_layer3_job().await,
        _ => {},
    }
}

async fn run_ai_worker() {
    println!("\n🤖 --- Layer 3 AI Worker ---");
    println!("Contribute GPU/CPU compute to decentralized AI inference jobs.\n");
    
    print!("Node RPC URL [http://127.0.0.1:9000]: ");
    io::stdout().flush().unwrap();
    let mut node_url = String::new();
    io::stdin().read_line(&mut node_url).unwrap();
    let node_url = if node_url.trim().is_empty() {
        "http://127.0.0.1:9000".to_string()
    } else {
        node_url.trim().to_string()
    };
    
    print!("Model ID [gpt-4o-mini]: ");
    io::stdout().flush().unwrap();
    let mut model_id = String::new();
    io::stdin().read_line(&mut model_id).unwrap();
    let model_id = if model_id.trim().is_empty() {
        "gpt-4o-mini".to_string()
    } else {
        model_id.trim().to_string()
    };
    
    println!("\n✅ Configuration:");
    println!("   Node: {}", node_url);
    println!("   Model: {}", model_id);
    println!("\n🔄 Worker Loop:");
    println!("   → Poll for pending AI jobs");
    println!("   → Compute PoW (anti-spam)");
    println!("   → Execute inference (GPU/CPU)");
    println!("   → Submit results, earn rewards");
    println!("\nPress Ctrl+C to stop.\n");
    
    let worker = crate::client::AiWorker::new(node_url, model_id);
    worker.start().await;
}
