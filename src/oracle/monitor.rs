use crate::crypto::KeyPair;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LtcTransaction {
    pub tx_hash: String,
    pub value: u64,  // satoshis
    pub confirmations: u32,
    pub received: String,  // timestamp
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

pub struct OracleMonitor {
    ltc_address: String,
    api_key: Option<String>,
    oracle_keypair: KeyPair,
    processed_txs: HashSet<String>,
    check_interval_secs: u64,
    min_confirmations: u32,
}

impl OracleMonitor {
    pub fn new(ltc_address: String, oracle_identity_path: &str) -> Self {
        // Load oracle keypair
        let oracle_keypair = if let Ok(data) = fs::read_to_string(oracle_identity_path) {
            // Parse as hex string
            let hex_str: String = serde_json::from_str(&data).expect("Failed to parse oracle identity file");
            let bytes = hex::decode(&hex_str).expect("Invalid hex in oracle identity");
            let key_bytes: [u8; 32] = bytes.try_into().expect("Invalid key length");
            KeyPair { signing_key: ed25519_dalek::SigningKey::from_bytes(&key_bytes) }
        } else {
            // Log via return or assume stderr implies console usage for now, 
            // but in GUI integration logging is passed via channel in run()
            // Here just do safe fallback or generate.
            let kp = KeyPair::generate();
            // Save as hex string
            let hex_str = hex::encode(kp.signing_key.to_bytes());
            let json = serde_json::to_string_pretty(&hex_str).unwrap();
            let _ = fs::write(oracle_identity_path, json);
            kp
        };

        // Load processed transactions
        let processed_txs = if let Ok(data) = fs::read_to_string("processed_ltc_txs.json") {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            HashSet::new()
        };

        Self {
            ltc_address,
            api_key: None,
            oracle_keypair,
            processed_txs,
            check_interval_secs: 60,
            min_confirmations: 6,
        }
    }

    pub fn get_public_key(&self) -> String {
        self.oracle_keypair.public_key_hex()
    }

    async fn check_deposits(&mut self, log_tx: &mpsc::Sender<String>) -> Result<Vec<LtcTransaction>, String> {
        let url = format!(
            "https://api.blockcypher.com/v1/ltc/main/addrs/{}",
            self.ltc_address
        );

        let _ = log_tx.send(format!("🔍 Checking LTC address: {}", self.ltc_address)).await;

        let response = reqwest::get(&url).await.map_err(|e| e.to_string())?;
        let address_data: BlockCypherAddress = response.json().await.map_err(|e| e.to_string())?;

        let mut new_deposits = Vec::new();

        if let Some(txrefs) = address_data.txrefs {
            for tx in txrefs {
                if tx.value <= 0 { continue; }
                if self.processed_txs.contains(&tx.tx_hash) { continue; }

                if tx.confirmations < self.min_confirmations {
                    let _ = log_tx.send(format!("⏳ TX {} has {} confirmations (need {})", 
                        &tx.tx_hash[..8], tx.confirmations, self.min_confirmations)).await;
                    continue;
                }

                new_deposits.push(LtcTransaction {
                    tx_hash: tx.tx_hash.clone(),
                    value: tx.value as u64,
                    confirmations: tx.confirmations,
                    received: tx.confirmed.unwrap_or_else(|| "unknown".to_string()),
                });

                self.processed_txs.insert(tx.tx_hash);
            }
        }

        if !new_deposits.is_empty() {
            let json = serde_json::to_string_pretty(&self.processed_txs).map_err(|e| e.to_string())?;
            let _ = fs::write("processed_ltc_txs.json", json);
        }

        Ok(new_deposits)
    }

    pub fn sign_deposit(&self, deposit: &LtcTransaction, user: &str, compass_collateral: u64, mint_amount: u64) -> String {
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

    pub async fn run(&mut self, log_tx: mpsc::Sender<String>, mut stop_rx: mpsc::Receiver<()>) {
        let _ = log_tx.send("🚀 Oracle Monitor Starting...".to_string()).await;
        let _ = log_tx.send(format!("   Monitoring: {}", self.ltc_address)).await;
        let _ = log_tx.send(format!("   PublicKey: {}", self.get_public_key())).await;

        loop {
            // Check for stop signal
            match stop_rx.try_recv() {
                Ok(_) | Err(mpsc::error::TryRecvError::Disconnected) => {
                    let _ = log_tx.send("🛑 Oracle Monitor Stopping...".to_string()).await;
                    break;
                }
                Err(mpsc::error::TryRecvError::Empty) => {} // Continue
            }

            match self.check_deposits(&log_tx).await {
                Ok(deposits) => {
                    if !deposits.is_empty() {
                        let _ = log_tx.send(format!("✅ Found {} new confirmed deposit(s)!", deposits.len())).await;
                        
                        for deposit in deposits {
                             let _ = log_tx.send(format!("💰 Deposit: {} sats (TX: {})", deposit.value, &deposit.tx_hash[..8])).await;
                            
                             // Auto-generate example signature for "Daniel"
                             let example_sig = self.sign_deposit(&deposit, "Daniel", deposit.value * 100, deposit.value / 100);
                             let _ = log_tx.send(format!("📝 Signature generated for Daniel (saved to file)")).await;

                             // Save signature
                             let sig_data = serde_json::json!({
                                "tx_hash": deposit.tx_hash,
                                "amount_satoshis": deposit.value,
                                "confirmations": deposit.confirmations,
                                "signature": example_sig,
                                "example_user": "Daniel",
                                "oracle_pubkey": self.get_public_key(),
                            });
                             let filename = format!("ltc_deposit_{}.json", &deposit.tx_hash[..8]);
                             let _ = fs::write(&filename, serde_json::to_string_pretty(&sig_data).unwrap());
                        }
                    }
                }
                Err(e) => {
                    let _ = log_tx.send(format!("❌ Error: {}", e)).await;
                }
            }

            // Sleep with interrupt check
            // Use select! to wait for either sleep or stop signal
            tokio::select! {
                _ = sleep(Duration::from_secs(self.check_interval_secs)) => {}
                _ = stop_rx.recv() => {
                     let _ = log_tx.send("🛑 Oracle Monitor Stopping...".to_string()).await;
                     break;
                }
            }
        }
    }
}
