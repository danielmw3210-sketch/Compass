use std::io::{self, Write};
use crate::cli::session::{Session, UserRole};
use crate::cli::keys::KeysCommands;
use std::sync::Arc;

/// PRODUCTION ENTRY POINT - Single Authentication, Role-Locked Menus
pub async fn start() {
    // ===== AUTHENTICATION GATE =====
    let session = match Session::authenticate() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("\n❌ {}", e);
            eprintln!("🔒 ACCESS DENIED. Exiting...\n");
            std::thread::sleep(std::time::Duration::from_secs(1));
            std::process::exit(1);
        }
    };
    
    println!("\n✅ Authenticated as: {:?} ({})", session.role, session.user_name);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    // ===== ROLE-SPECIFIC MENU (NO SWITCHING) =====
    match session.role {
        UserRole::Admin => admin_menu(session).await,
        UserRole::Worker => worker_menu(session).await,
        UserRole::Client => client_menu(session).await,
    }
    
    // ===== CLEAN EXIT =====
    println!("\nSession ended. Goodbye.");
    std::process::exit(0);
}

// ========== ADMIN MENU ==========
async fn admin_menu(session: Session) {
    loop {
        print_header("ADMIN CONTROL PANEL");
        println!("1. Start Admin Node (Leader)");
        println!("2. Tools (Wipe DB, Init Oracle)");
        println!("3. View System Wallets (Debug)");
        println!("4. Generate Keys");
        println!("5. Exit");
        print!("\nSelect: ");
        io::stdout().flush().unwrap();
        
        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();
        
        match choice.trim() {
            "1" => run_admin_node(session.identity.clone().expect("Admin must have identity")).await,
            "2" => tools_menu().await,
            "3" => {
                println!("\n[System Wallets - Admin View]");
                println!("(This would call RPC to list all wallets)");
                pause();
            },
            "4" => key_menu(),
            "5" => break,
            _ => println!("Invalid option."),
        }
    }
}

// ========== WORKER MENU ==========
async fn worker_menu(session: Session) {
    print_header("WORKER / VERIFIER PANEL");
    println!("Worker Role: Compute jobs only");
    println!("🔒 Wallet operations DISABLED for security\n");
    
    println!("1. Start Oracle Verification Jobs");
    println!("2. Start AI Compute Worker");
    println!("3. View My Stats (Coming Soon)");
    println!("4. Exit");
    print!("\nSelect: ");
    io::stdout().flush().unwrap();
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();
    
    match choice.trim() {
        "1" => run_oracle_verification_worker().await,
        "2" => run_ai_worker().await,
        "3" => {
            println!("\n[Worker Stats - Coming Soon]");
            pause();
        },
        "4" => {},
        _ => println!("Invalid option."),
    }
}

// ========== CLIENT MENU ==========
async fn client_menu(session: Session) {
    loop {
        print_header(&format!("CLIENT WALLET - {}", session.user_name));
        
        if !session.authenticated {
            println!("⚠️  Read-only mode (not authenticated)");
        }
        
        println!("1. View Balance");
        println!("2. Transfer Funds");
        println!("3. Mint Compass (Collateral)");
        println!("4. Burn Compass (Redeem)");
        println!("5. Buy Neural Network");
        println!("6. View All NFTs (Debug)");
        println!("7. Exit");
        print!("\nSelect: ");
        io::stdout().flush().unwrap();
        
        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();
        
        match choice.trim() {
            "1" => {
                // 1. Connect to RPC
                print!("Node URL [http://127.0.0.1:9000]: ");
                io::stdout().flush().unwrap();
                let mut node_url = String::new();
                io::stdin().read_line(&mut node_url).unwrap();
                let node_url = if node_url.trim().is_empty() {
                    "http://127.0.0.1:9000".to_string()
                } else {
                    node_url.trim().to_string()
                };
                
                let client = crate::client::RpcClient::new(node_url);

                // 2. Determine Address
                let address = if let Some(id) = &session.identity {
                     hex::encode(id.signing_key.verifying_key().as_bytes())
                } else {
                     print!("Enter Wallet Address: ");
                     io::stdout().flush().unwrap();
                     let mut addr = String::new();
                     io::stdin().read_line(&mut addr).unwrap();
                     addr.trim().to_string()
                };

                if address.is_empty() {
                    println!("❌ No address provided.");
                } else {
                    println!("\n📡 Connecting to network...");
                    match client.get_balance(&address, "COMPASS").await {
                         Ok(bal) => println!("💳 Balance: {} COMPASS", bal),
                         Err(e) => println!("❌ Network Error: {}", e),
                    }
                    // Also check for COMPUTE credits
                    match client.get_balance(&address, "COMPUTE").await {
                        Ok(bal) if bal > 0 => println!("🧠 Credits: {} COMPUTE", bal),
                        _ => {}
                    }
                }
                pause();
            },
            "2" => {
                if !session.can_make_transfers() {
                    println!("\n❌ You must be authenticated to make transfers.");
                    pause();
                    continue;
                }
                println!("\n[Transfer Interface]");
                println!("(Would call submitTransfer RPC)");
                pause();
            },
            "3" => {
                if !session.can_make_transfers() {
                    println!("\n❌ You must be authenticated to mint.");
                    pause();
                    continue;
                }
                println!("\n[Mint Compass]");
                println!("(Would call submitMint RPC)");
                pause();
            },
            "4" => {
                if !session.can_make_transfers() {
                    println!("\n❌ You must be authenticated to burn.");
                    pause();
                    continue;
                }
                println!("\n[Burn Compass]");
                println!("(Would call submitBurn RPC)");
                pause();
            },
            "5" => {
                crate::layer3::user_ops::run_user_ai_menu().await;
            },
            "6" => {
                 // VIEW ALL NFTS (DEBUG)
                print!("Node URL [http://127.0.0.1:9000]: ");
                io::stdout().flush().unwrap();
                let mut node_url = String::new();
                io::stdin().read_line(&mut node_url).unwrap();
                let node_url = if node_url.trim().is_empty() { "http://127.0.0.1:9000".to_string() } else { node_url.trim().to_string() };
                
                let client = crate::client::RpcClient::new(node_url);
                match client.get_all_nfts().await {
                    Ok(nfts) => {
                        println!("\n🎨 --- ALL PERSISTED NFTS ({}) ---", nfts.len());
                        for nft in nfts {
                            println!("- [{}] {}", nft.minted_at, nft.token_id);
                            println!("  Name: {}", nft.name);
                            println!("  Owner: {}", nft.current_owner);
                            println!("  Accuracy: {:.2}%", nft.accuracy * 100.0);
                            println!("  Source: {}", if nft.token_id.contains("NN_MODEL") { "RocksDB (New)" } else { "JSON (Legacy)" });
                            println!("--------------------------------------------------");
                        }
                    },
                    Err(e) => println!("❌ Error fetching NFTs: {}", e),
                }
                pause();
            },
            "7" => break,
            _ => println!("Invalid option."),
        }
    }
}

// ========== HELPER FUNCTIONS ==========

fn print_header(title: &str) {
    print!("\x1B[2J\x1B[1;1H");
    println!("========================================");
    println!("  {}", title);
    println!("========================================\n");
}

fn pause() {
    println!("\nPress Enter to continue...");
    let mut _pause = String::new();
    io::stdin().read_line(&mut _pause).unwrap();
}

async fn run_admin_node(identity: Arc<crate::crypto::KeyPair>) {
    println!("\n🚀 Starting Admin Node...");
    
    let mut leader_config = crate::config::CompassConfig::default();
    leader_config.node.rpc_port = 9000;
    leader_config.node.db_path = "compass_db_leader".to_string();
    
    crate::run_node_mode_internal(
        leader_config,
        None,
        Some(identity),
    ).await;
}

async fn run_oracle_verification_worker() {
    println!("\n🔍 Starting Oracle Verification Worker...\n");
    
    // Use new robust helper
    let node_url = prompt_node_url("http://localhost:9000");
    
    if let Err(e) = crate::worker_menu::worker_job_menu(&node_url).await {
        println!("Worker error: {}", e);
        pause();
    }
}

async fn run_ai_worker() {
    println!("\n🤖 Starting AI Worker...\n");
    
    let node_url = prompt_node_url("http://127.0.0.1:9000");
    
    print!("Model ID [gpt-4o-mini]: ");
    io::stdout().flush().unwrap();
    let mut model_id = String::new();
    io::stdin().read_line(&mut model_id).unwrap();
    let model_id = if model_id.trim().is_empty() {
        "gpt-4o-mini".to_string()
    } else {
        model_id.trim().to_string()
    };
    
    // Load Identity
    let identity = match load_and_maybe_create_identity("worker") { // Default to 'worker' wallet
        Some(id) => id,
        None => return,
    };
    
    // Decrypt
    let keypair = match identity.into_keypair() {
        Ok(kp) => kp,
        Err(e) => {
            println!("❌ Failed to decrypt key: {}", e);
            pause();
            return;
        }
    };
    
    let worker = crate::client::AiWorker::new(node_url, model_id, keypair);
    worker.start().await;
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
        "1" => crate::cli::keys::handle_keys_command(KeysCommands::Generate { 
            role: "admin".to_string(), 
            name: "admin".to_string() 
        }),
        "2" => crate::cli::keys::handle_keys_command(KeysCommands::Generate { 
            role: "verifier".to_string(), 
            name: "verifier".to_string() 
        }),
        "3" => {
            print!("Enter user name: ");
            io::stdout().flush().unwrap();
            let mut name = String::new();
            io::stdin().read_line(&mut name).unwrap();
            crate::cli::keys::handle_keys_command(KeysCommands::Generate { 
                role: "user".to_string(), 
                name: name.trim().to_string() 
            });
        },
        "4" => {
            print!("Enter identity name: ");
            io::stdout().flush().unwrap();
            let mut name = String::new();
            io::stdin().read_line(&mut name).unwrap();
            crate::cli::keys::handle_keys_command(KeysCommands::ExportPub { 
                name: name.trim().to_string() 
            });
        },
        _ => {},
    }
    pause();
}

async fn tools_menu() {
    println!("\n--- Admin Tools ---");
    println!("1. Wipe Leader DB");
    println!("2. Wipe Verifier DB");
    println!("3. Generate Admin Genesis Config");
    println!("4. Init Finance Oracle");
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
        "4" => {
            println!("\n🚀 Initializing Layer 3 Finance Oracle...");
            let client = crate::client::RpcClient::new("http://127.0.0.1:9000".to_string());
            match crate::layer3::finance_oracle::init_finance_oracle_job(&client).await {
                Ok(_) => println!("✓ Initialization complete."),
                Err(e) => println!("❌ Error: {}", e),
            }
        },
        _ => {},
    }
    pause();
}

// ========== PUBLIC UTILITY FUNCTIONS ==========
// (Used by worker_menu.rs and layer3/user_ops.rs)

/// Load identity for backward compatibility with other modules


pub fn load_identity(name: &str) -> Option<crate::identity::Identity> {
    use std::path::Path;
    
    let filename = format!("{}.json", name);
    if !Path::new(&filename).exists() {
        return None;
    }
    
    print!("Enter password for '{}': ", name);
    io::stdout().flush().unwrap();
    let mut pass = String::new();
    io::stdin().read_line(&mut pass).unwrap();
    
    match crate::identity::Identity::load_and_decrypt(Path::new(&filename), pass.trim()) {
        Ok(id) => Some(id),
        Err(e) => {
            println!("Authentication Failed: {}", e);
            None
        }
    }
}

/// Robustly prompt for Node URL with defaults and validation
pub fn prompt_node_url(default: &str) -> String {
    print!("Node URL [{}]: ", default);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let url = input.trim();
    if url.is_empty() {
        default.to_string()
    } else {
        // Basic fixup
        if !url.starts_with("http") {
             format!("http://{}", url)
        } else {
             url.to_string()
        }
    }
}

/// Load identity or prompt to create one if it doesn't exist
pub fn load_and_maybe_create_identity(default_name: &str) -> Option<crate::identity::Identity> {
    let mut name = String::new();
    print!("   Enter wallet name (default: '{}'): ", default_name);
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut name).unwrap();
    let name = if name.trim().is_empty() { default_name } else { name.trim() };
    
    if let Some(id) = load_identity(name) {
        return Some(id);
    }
    
    // Not found - Prompt to create
    println!("⚠️  Wallet '{}' not found.", name);
    print!("   Create new wallet '{}'? (Y/n): ", name);
    io::stdout().flush().unwrap();
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();
    
    if choice.trim().eq_ignore_ascii_case("n") {
        return None;
    }
    
    // Create logic
    println!("   Creating new wallet '{}'...", name);
    // Let's prompt for password to be secure
    print!("   Set password: ");
    io::stdout().flush().unwrap();
    let mut pass = String::new();
    io::stdin().read_line(&mut pass).unwrap();
    let pass = pass.trim();
    
    // Create new identity (User role by default for workers)
    let (id, mnemonic) = match crate::identity::Identity::new(name, crate::identity::NodeRole::User, pass) {
        Ok(v) => v,
        Err(e) => {
            println!("❌ Failed to generate identity: {}", e);
            return None;
        }
    };
    
    println!("📝 Write down your mnemonic phrase (SAFE KEEPING in admin_key.mnemonic style):");
    println!("{}", mnemonic); // Production should handle this better, but improved for now.

    let path_str = format!("{}.json", name);
    if let Err(e) = id.save(std::path::Path::new(&path_str)) {
        println!("❌ Failed to save wallet: {}", e);
        return None;
    }
    
    println!("✅ Wallet '{}' created!", name);
    println!("   Address: {}", id.public_key);
    Some(id)
}
