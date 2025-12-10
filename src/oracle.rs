use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::vault::VaultManager;
use crate::crypto::KeyPair;

const LTC_API_URL: &str = "https://api.blockcypher.com/v1/ltc/main/addrs";

#[derive(Deserialize, Debug)]
struct TxRef {
    tx_hash: String,
    block_height: i64,
    tx_input_n: i64,
    tx_output_n: i64,
    value: u64, // Satoshis
    ref_balance: i64,
    confirmations: i64,
    double_spend: bool,
}

#[derive(Deserialize, Debug)]
struct AddressFull {
    address: String,
    total_received: u64,
    total_sent: u64,
    balance: u64,
    unconfirmed_balance: u64,
    final_balance: u64,
    n_tx: i64,
    unconfirmed_n_tx: i64,
    final_n_tx: i64,
    txrefs: Option<Vec<TxRef>>,
}

pub struct OracleService {
    target_address: String,
    admin_key: Arc<KeyPair>,
    processed_txs: HashSet<String>,
}

impl OracleService {
    pub fn new(address: &str, admin_key: Arc<KeyPair>) -> Self {
        Self {
            target_address: address.to_string(),
            admin_key,
            processed_txs: HashSet::new(),
        }
    }

    /// Fetches confirmed deposits from BlockCypher
    pub async fn check_deposits(&mut self) -> Vec<(String, u64)> {
        let url = format!("{}/{}", LTC_API_URL, self.target_address);
        
        let client = reqwest::Client::new();
        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                println!("[ORACLE] Network Error: {}", e);
                return vec![];
            }
        };

        let data: AddressFull = match resp.json().await {
            Ok(d) => d,
            Err(e) => {
                println!("[ORACLE] JSON Parse Error: {}", e);
                return vec![];
            }
        };

        let mut new_deposits = Vec::new();

        if let Some(txs) = data.txrefs {
            for tx in txs {
                // Criteria:
                // 1. Confirmed (> 1 confirmation)
                // 2. Incoming (value usually implies output to this addr, but txrefs structure
                //    in BlockCypher lists operations. We need to check if it increased balance.)
                //    Actually, 'tx_input_n' = -1 means it's an output TO this address.
                //    'tx_output_n' = -1 means it's an input FROM this address.
                //    Let's stick to simple 'confirmations > 1' and 'not processed'.
                //    Simplified: Assume positive value means received for now (BlockCypher specific parsing needed).
                //    Wait, checking docs: tx_input_n is input index, if -1 it's not an input (so it's an output TO us).
                
                if tx.confirmations > 0 && tx.tx_input_n == -1 && !self.processed_txs.contains(&tx.tx_hash) {
                    println!("[ORACLE] Found CONFIRMED deposit: {} sats (TX: {})", tx.value, tx.tx_hash);
                    new_deposits.push((tx.tx_hash.clone(), tx.value));
                    self.processed_txs.insert(tx.tx_hash);
                }
            }
        }

        new_deposits
    }

    /// Sign a proof for the deposit + user intent
    pub fn sign_mint_proof(&self, collateral: &str, amount: u64, tx_hash: &str, mint_amount: u64, owner: &str) -> String {
        // Msg: "DEPOSIT:LTC:5000:0xHash:100:Daniel"
        let msg = format!("DEPOSIT:{}:{}:{}:{}:{}", collateral, amount, tx_hash, mint_amount, owner);
        self.admin_key.sign_hex(msg.as_bytes())
    }
}
