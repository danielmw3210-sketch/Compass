mod crypto;
mod block;
mod chain;
mod wallet;
mod vdf;

mod network;
use network::{NetMessage, start_server, connect_and_send};
use crypto::KeyPair;
use block::{
    create_poh_block, create_vote_block,
    create_proposal_block, current_unix_timestamp_ms,
};

use chain::Chain;
use wallet::{WalletManager, WalletType};
use std::io::{self, Write};
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::fs::OpenOptions;

/// Simple file logger
fn log_to_file(msg: &str) {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("compass.log")
        .unwrap();
    writeln!(file, "{}", msg).unwrap();
}

#[tokio::main]
async fn main() {
    // --- Setup admin ---
    let admin = Arc::new(KeyPair::new());
    let admin_wallet_id = "admin".to_string();
    let admin_pubkey_hex = admin.public_key_hex();

    // --- Load wallets ---
    let mut wallet_manager = WalletManager::load("wallets.json");
    if wallet_manager.get_wallet("Daniel").is_none() {
        let admin_wallet = wallet::Wallet::new("Daniel", WalletType::Admin);
        wallet_manager.wallets.push(admin_wallet);
        wallet_manager.save("wallets.json");
        println!("Created admin wallet for Daniel");
    }

    // --- Load chain ---
    let chain = Arc::new(Mutex::new(Chain::load_from_json("compass_chain.json")));
    let wallets = Arc::new(Mutex::new(wallet_manager));

    // --- Genesis Mint (from Foundation Reserve) ---
    {
        let mut wallets = wallets.lock().unwrap();
        if wallets.get_balance("Daniel", "Compass") == 0 {
            wallets.credit("Daniel", "Compass", 100_000);
            wallets.save("wallets.json");
            log_to_file("GENESIS: Minted 100,000 Compass to Daniel (Unbacked Foundation Reserve)");
            println!("GENESIS: Minted 100,000 Compass to Daniel");
        }
    }

    // --- Spawn PoH + VDF loop ---
    {
        let chain = Arc::clone(&chain);
        // let wallets = Arc::clone(&wallets); // Unused in loop now
        let admin = Arc::clone(&admin);
        // let admin_wallet_id = admin_wallet_id.clone(); // Unused in loop

        thread::spawn(move || {
            let mut tick: u64 = 0;
            let mut seed = b"COMPASS_GENESIS_SEED".to_vec();
            
            // Restore state from Chain if exists
            {
                let chain_guard = chain.lock().unwrap();
                // Find last PoH block
                for block in chain_guard.blocks.iter().rev() {
                    if let block::BlockType::PoH { tick: last_tick, hash: ref last_hash, .. } = block.block_type {
                        tick = last_tick;
                        if let Ok(decoded_hash) = hex::decode(last_hash) {
                            seed = decoded_hash;
                        }
                        println!("Restored PoH State: Tick={}, VDF Hash={}", tick, last_hash);
                        break;
                    }
                }
            }

            let mut vdf_state = vdf::VDFState::new(seed);
            
            // Target: ~400ms per block (Solana style)
            // Based on logs: ~290k H/s on this machine.
            // 120,000 / 290,000 ~= 0.41s
            let iterations_per_tick = 120_000;

            loop {
                // Run VDF Work (CPU intensive!)
                let start = std::time::Instant::now();
                let start_hash = vdf_state.current_hash.clone();
                let end_hash = vdf_state.execute(iterations_per_tick);
                let duration = start.elapsed();

                {
                    let mut chain = chain.lock().unwrap();

                    // Bootstrap genesis if empty (rarely happens here if we restored, but good for clean start)
                    if chain.blocks.is_empty() {
                         let genesis = create_poh_block(
                            "GENESIS".to_string(), 
                            0, 
                            0, 
                            start_hash.clone(), // initial seed
                            &admin
                        );
                        chain.blocks.push(genesis);
                        chain.save_to_json("compass_chain.json").unwrap();
                        println!("Genesis PoH block created.");
                    }

                    // PoH tick (signed by admin)
                    tick += 1;
                    let poh = create_poh_block(
                        chain.head_hash().unwrap_or_default(),
                        tick,
                        iterations_per_tick,
                        end_hash.clone(),
                        &admin,
                    );
                    chain.blocks.push(poh);
                }

                // Log periodically (outside lock)
                if tick % 10 == 0 || tick == 1 {
                    let chain = chain.lock().unwrap();
                    let millis = duration.as_millis();
                    // Avoid divide by zero
                    let hps = if millis > 0 { (iterations_per_tick as u128 * 1000) / millis } else { 0 };
                    
                    println!("PoH Tick #{}: {} hashes in {}ms (~{} H/s)", 
                        tick, iterations_per_tick, millis, hps);
                    log_to_file(&format!("PoH Tick #{}: {}ms (~{} H/s)", tick, millis, hps));
                    chain.save_to_json("compass_chain.json").unwrap();
                }
                
                // Sleep tiny amount to prevent log flooding only, in prod this is 0
                thread::sleep(Duration::from_millis(10));
            }
        });
    }

    // --- Networking Setup ---
    tokio::spawn(start_server("0.0.0.0:9000"));

    tokio::spawn(async {
        tokio::time::sleep(Duration::from_secs(2)).await;
        connect_and_send("127.0.0.1:9000", NetMessage::Ping).await;
    });

    // --- Admin Menu Loop (Main Thread) ---
    loop {
        println!("\n=== Compass Admin Menu ===");
        println!("1. Show status");
        println!("2. Create proposal");
        println!("3. Cast vote");
        println!("4. Tally votes");
        println!("5. Exit");
        print!("Select an option: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let choice = input.trim();

        match choice {
            "1" => {
                let chain = chain.lock().unwrap();
                let wallets = wallets.lock().unwrap();
                println!("Chain size: {}", chain.blocks.len());
                for w in &wallets.wallets {
                    println!("{}: {:?} ({:?})", w.owner, w.balances, w.wallet_type);
                }
            }
            "2" => {
                print!("Enter proposal text: ");
                io::stdout().flush().unwrap();
                let mut text = String::new();
                io::stdin().read_line(&mut text).unwrap();

                print!("Enter deadline (unix ms): ");
                io::stdout().flush().unwrap();
                let mut deadline_str = String::new();
                io::stdin().read_line(&mut deadline_str).unwrap();
                let deadline: u64 = deadline_str.trim().parse().unwrap();

                let mut chain = chain.lock().unwrap();
                let proposal = create_proposal_block(
                    admin_wallet_id.clone(),
                    text.trim().to_string(),
                    deadline,
                    chain.head_hash(),
                    |msg| admin.sign_hex(msg),
                    || current_unix_timestamp_ms(),
                    |id| chain.proposal_id_exists(id),
                ).expect("proposal failed");
                chain.append_proposal(proposal, &admin_pubkey_hex).expect("append proposal failed");
                println!("Proposal appended.");
            }
            "3" => {
                print!("Enter proposal ID: ");
                io::stdout().flush().unwrap();
                let mut id_str = String::new();
                io::stdin().read_line(&mut id_str).unwrap();
                let proposal_id: u64 = id_str.trim().parse().unwrap();

                print!("Vote yes/no: ");
                io::stdout().flush().unwrap();
                let mut vote_str = String::new();
                io::stdin().read_line(&mut vote_str).unwrap();
                let choice = vote_str.trim().eq_ignore_ascii_case("yes");

                let voter = KeyPair::new();
                let mut chain = chain.lock().unwrap();
                let vote_block = create_vote_block(
                    "Daniel".to_string(),
                    proposal_id,
                    choice,
                    chain.head_hash(),
                    &voter,
                );
                chain.append_vote(vote_block, &voter.public_key_hex()).expect("append vote failed");
                println!("Vote appended.");
            }
            "4" => {
                print!("Enter proposal ID: ");
                io::stdout().flush().unwrap();
                let mut id_str = String::new();
                io::stdin().read_line(&mut id_str).unwrap();
                let proposal_id: u64 = id_str.trim().parse().unwrap();

                let chain = chain.lock().unwrap();
                let (yes, no) = chain.tally_votes(proposal_id);
                println!("Proposal {} tally: YES={} NO={}", proposal_id, yes, no);
            }
            "5" => {
                println!("Shutting down Compass...");
                break;
            }
            _ => println!("Invalid option, try again."),
        }
    }
}