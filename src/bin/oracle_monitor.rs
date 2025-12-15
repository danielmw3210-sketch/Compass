use rust_compass::crypto::KeyPair;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use tokio::time::{sleep, Duration};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LtcTransaction {
    tx_hash: String,
    value: u64,  // satoshis
    confirmations: u32,
    received: String,  // timestamp
}

#[derive(Serialize, Deserialize, Debug)]
struct BlockCypherAddress {
    address: String,
    total_received: u64,
    balance: u64,
    unconfirmed_balance: u64,
    txrefs: Option<Vec<TxRef>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TxRef {
    tx_hash: String,
    value: i64,  // Can be negative for outgoing
    confirmations: u32,
    confirmed: Option<String>,
}

struct OracleMonitor {
    ltc_address: String,
    api_key: Option<String>,
    oracle_keypair: KeyPair,
    processed_txs: HashSet<String>,
    check_interval_secs: u64,
    min_confirmations: u32,
}

impl OracleMonitor {
    fn new(ltc_address: String, oracle_identity_path: &str) -> Self {
        // Load oracle keypair
        let oracle_keypair = if let Ok(data) = fs::read_to_string(oracle_identity_path) {
            // Parse as hex string
            let hex_str: String = serde_json::from_str(&data).expect("Failed to parse oracle identity file");
            let bytes = hex::decode(&hex_str).expect("Invalid hex in oracle identity");
            let key_bytes: [u8; 32] = bytes.try_into().expect("Invalid key length");
            KeyPair { signing_key: ed25519_dalek::SigningKey::from_bytes(&key_bytes) }
        } else {
            eprintln!("⚠️  Oracle identity not found at {}, generating new one...", oracle_identity_path);
            let kp = KeyPair::generate();
            // Save as hex string
            let hex_str = hex::encode(kp.signing_key.to_bytes());
            let json = serde_json::to_string_pretty(&hex_str).unwrap();
            fs::write(oracle_identity_path, json).expect("Failed to save oracle identity");
            kp
        };

        println!("🔑 Oracle Public Key: {}", oracle_keypair.public_key_hex());
        
        // Load processed transactions from file
        let processed_txs = if let Ok(data) = fs::read_to_string("processed_ltc_txs.json") {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            HashSet::new()
        };

        Self {
            ltc_address,
            api_key: None,  // Can add BlockCypher API key for higher limits
            oracle_keypair,
            processed_txs,
            check_interval_secs: 60,  // Check every 60 seconds
            min_confirmations: 6,     // Wait for 6 confirmations
        }
    }

    async fn check_deposits(&mut self) -> Result<Vec<LtcTransaction>, Box<dyn std::error::Error>> {
        let url = format!(
            "https://api.blockcypher.com/v1/ltc/main/addrs/{}",
            self.ltc_address
        );

        println!("🔍 Checking LTC address: {}", self.ltc_address);

        let response = reqwest::get(&url).await?;
        let address_data: BlockCypherAddress = response.json().await?;

        let mut new_deposits = Vec::new();

        if let Some(txrefs) = address_data.txrefs {
            for tx in txrefs {
                // Only process incoming transactions
                if tx.value <= 0 {
                    continue;
                }

                // Skip if already processed
                if self.processed_txs.contains(&tx.tx_hash) {
                    continue;
                }

                // Check confirmations
                if tx.confirmations < self.min_confirmations {
                    println!("⏳ TX {} has {} confirmations (need {})", 
                        &tx.tx_hash[..8], tx.confirmations, self.min_confirmations);
                    continue;
                }

                // New confirmed deposit!
                new_deposits.push(LtcTransaction {
                    tx_hash: tx.tx_hash.clone(),
                    value: tx.value as u64,
                    confirmations: tx.confirmations,
                    received: tx.confirmed.unwrap_or_else(|| "unknown".to_string()),
                });

                self.processed_txs.insert(tx.tx_hash);
            }
        }

        // Save processed transactions
        if !new_deposits.is_empty() {
            let json = serde_json::to_string_pretty(&self.processed_txs)?;
            fs::write("processed_ltc_txs.json", json)?;
        }

        Ok(new_deposits)
    }

    fn sign_deposit(&self, deposit: &LtcTransaction, user: &str, compass_collateral: u64, mint_amount: u64) -> String {
        // Format: "NATIVE_DEPOSIT:{payment_asset}:{payment_amount}:{tx_hash}:{compass_collateral}:{requested_mint_amount}:{owner}"
        let message = format!(
            "NATIVE_DEPOSIT:LTC:{}:{}:{}:{}:{}",
            deposit.value,
            deposit.tx_hash,
            compass_collateral,
            mint_amount,
            user
        );

        let signature = self.oracle_keypair.sign(message.as_bytes());
        hex::encode(signature.to_bytes())
    }

    async fn run(&mut self) {
        println!("🚀 Oracle Monitor Starting...");
        println!("   Monitoring LTC address: {}", self.ltc_address);
        println!("   Check interval: {}s", self.check_interval_secs);
        println!("   Min confirmations: {}", self.min_confirmations);
        println!();

        loop {
            match self.check_deposits().await {
                Ok(deposits) => {
                    if !deposits.is_empty() {
                        println!("\n✅ Found {} new confirmed deposit(s)!", deposits.len());
                        
                        for deposit in deposits {
                            println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                            println!("💰 LTC Deposit Detected!");
                            println!("   TX Hash: {}", deposit.tx_hash);
                            println!("   Amount: {} satoshis (0.{:08} LTC)", deposit.value, deposit.value);
                            println!("   Confirmations: {}", deposit.confirmations);
                            println!("   Time: {}", deposit.received);
                            println!();
                            
                            // Example signature - user would provide collateral/mint amounts
                            let example_sig = self.sign_deposit(&deposit, "Daniel", deposit.value * 100, deposit.value / 100);
                            
                            println!("📝 Example Signature (100:1 ratio):");
                            println!("   User: Daniel");
                            println!("   Collateral: {} COMPASS", deposit.value * 100);
                            println!("   Mint: {} Compass-LTC", deposit.value / 100);
                            println!();
                            println!("🔐 Oracle Signature:");
                            println!("   {}", example_sig);
                            println!();
                            println!("📋 Copy this into GUI Vaults page:");
                            println!("   TX Hash: {}", deposit.tx_hash);
                            println!("   Signature: {}", example_sig);
                            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

                            // Save signature to file for easy access
                            let sig_data = serde_json::json!({
                                "tx_hash": deposit.tx_hash,
                                "amount_satoshis": deposit.value,
                                "confirmations": deposit.confirmations,
                                "signature": example_sig,
                                "example_user": "Daniel",
                                "example_collateral": deposit.value * 100,
                                "example_mint": deposit.value / 100,
                                "oracle_pubkey": self.oracle_keypair.public_key_hex(),
                            });

                            let filename = format!("ltc_deposit_{}.json", &deposit.tx_hash[..8]);
                            fs::write(&filename, serde_json::to_string_pretty(&sig_data).unwrap())
                                .expect("Failed to save signature");
                            println!("💾 Saved to: {}", filename);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error checking deposits: {}", e);
                }
            }

            sleep(Duration::from_secs(self.check_interval_secs)).await;
        }
    }
}

#[tokio::main]
async fn main() {
    println!("⚡ Compass Oracle Monitor");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // TODO: Replace with your actual LTC address
    let ltc_address = std::env::var("LTC_ADMIN_ADDRESS")
        .unwrap_or_else(|_| {
            println!("⚠️  LTC_ADMIN_ADDRESS not set, using example address");
            "LTC_EXAMPLE_ADDRESS".to_string()
        });

    let mut monitor = OracleMonitor::new(ltc_address, "oracle.json");
    monitor.run().await;
}
