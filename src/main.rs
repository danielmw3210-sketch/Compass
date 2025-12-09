mod crypto;
mod block;
mod chain;
mod wallet;

mod network; // bring in your network.rs

use crypto::KeyPair;
use block::{
    create_poh_block, create_reward_block, create_vote_block,
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

#[tokio::main] // async runtime for networking + background tasks
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

    // --- Gul
 
    // --- Spawn PoH + reward loop ---
    {
        let chain = Arc::clone(&chain);
        let wallets = Arc::clone(&wallets);
        let admin = Arc::clone(&admin);
        let admin_wallet_id = admin_wallet_id.clone();
        let admin_pubkey_hex = admin_pubkey_hex.clone();

        thread::spawn(move || {
            let mut tick: u64 = 0;

            loop {
                {
                    let mut chain = chain.lock().unwrap();

                    // Bootstrap genesis if empty
                    if chain.blocks.is_empty() {
                        let genesis = create_poh_block("GENESIS".to_string(), 0, &admin);
                        chain.blocks.push(genesis);
                        chain.save_to_json("compass_chain.json").unwrap();
                        println!("Genesis PoH block created.");
                    }

                    // PoH tick (signed by admin)
                    tick += 1;
                    let poh = create_poh_block(
                        chain.head_hash().unwrap_or_default(),
                        tick,
                        &admin,
                    );
                    chain.blocks.push(poh);

                    // Reward (signed by admin)
                    let reward = create_reward_block(
                        admin_wallet_id.clone(),
                        "Daniel".to_string(),
                        1,
                        format!("PoH tick {}", tick),
                        chain.head_hash(),
                        &admin,
                    );
                    chain.append_reward(reward, &admin_pubkey_hex).expect("append reward failed");
                }

                {
                    let mut wallets = wallets.lock().unwrap();
                    wallets.credit("Daniel", 1);
                }

                // --- Logging + saving outside locks ---
                {
                    let chain = chain.lock().unwrap();
                    log_to_file(&format!(
                        "[CHAIN] PoH tick appended. Tick: {} Chain size: {}",
                        tick, chain.blocks.len()
                    ));
                    chain.save_to_json("compass_chain.json").unwrap();
                }
                {
                    let wallets = wallets.lock().unwrap();
                    wallets.save("wallets.json");
                    log_to_file(&format!(
                        "[CHAIN] Minted 1 coin to Daniel. New balance: {}",
                        wallets.get_wallet("Daniel").unwrap().balance
                    ));
                }

                thread::sleep(Duration::from_secs(30));
            }
        });
    }

    // --- Admin menu loop ---
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
                    println!("{}: {} coins ({:?})", w.owner, w.balance, w.wallet_type);
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

                let voter = KeyPair::new(); // later: load from wallet
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