use crate::crypto::KeyPair;
use crate::oracle::chains::{BitcoinClient, LitecoinClient};
use crate::oracle::types::{DepositProof, DepositRequest, OracleConfig};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use crate::layer3::models::BridgePredictor;
use crate::layer2::Layer2State;
use crate::layer3::data::FinanceDataFetcher;

pub struct OracleService {
    config: OracleConfig,
    btc_client: BitcoinClient,
    ltc_client: LitecoinClient,
    oracle_keypair: KeyPair,
    processed_deposits: HashSet<String>,
    
    // Bridge Components
    predictor: BridgePredictor,
    fetcher: FinanceDataFetcher,
    layer2: Arc<Mutex<Layer2State>>,
}

impl OracleService {
    pub fn new(config: OracleConfig, oracle_keypair: KeyPair, layer2: Arc<Mutex<Layer2State>>) -> Self {
        let btc_client = BitcoinClient::new(config.blockcypher_api_key.clone());
        let ltc_client = LitecoinClient::new(config.blockcypher_api_key.clone());
        
        // Initialize AI components
        println!("[Oracle] Initializing Bridge Neural Network...");
        let predictor = BridgePredictor::new();
        let fetcher = FinanceDataFetcher::new();

        Self {
            config,
            btc_client,
            ltc_client,
            oracle_keypair,
            processed_deposits: HashSet::new(),
            predictor,
            fetcher,
            layer2,
        }
    }

    pub async fn verify_deposit(&mut self, request: DepositRequest) -> Result<DepositProof, String> {
        // Check if already processed
        if self.processed_deposits.contains(&request.tx_hash) {
            return Err("Deposit already processed".to_string());
        }

        match request.chain.as_str() {
            "BTC" => self.verify_btc_deposit(request).await,
            "LTC" => self.verify_ltc_deposit(request).await,
            "SOL" => Err("SOL verification not yet implemented".to_string()),
            _ => Err(format!("Unsupported chain: {}", request.chain)),
        }
    }

    async fn verify_btc_deposit(&mut self, request: DepositRequest) -> Result<DepositProof, String> {
        println!("[Oracle] Verifying BTC deposit: {}", request.tx_hash);

        // Verify on Bitcoin blockchain via BlockCypher
        let (verified, confirmations) = self.btc_client
            .verify_deposit(&request.tx_hash, &request.vault_address, request.expected_amount)
            .await?;

        if !verified {
            return Err("Deposit not found or insufficient amount".to_string());
        }

        // Check confirmations
        if confirmations < self.config.min_confirmations_btc {
            return Err(format!(
                "Insufficient confirmations: {} (need {})",
                confirmations, self.config.min_confirmations_btc
            ));
        }

        println!("[Oracle] ✓ Verified: {} confirmations", confirmations);

        // Generate Oracle signature
        let message = format!(
            "DEPOSIT:{}:{}:{}:{}:{}",
            request.chain,
            request.expected_amount,
            request.tx_hash,
            request.expected_amount, // mint_amount (same for now)
            request.requester
        );

        let oracle_sig = self.oracle_keypair.sign_hex(message.as_bytes());
        let oracle_pubkey = self.oracle_keypair.public_key_hex();

        // Mark as processed
        self.processed_deposits.insert(request.tx_hash.clone());

        Ok(DepositProof {
            verified: true,
            tx_hash: request.tx_hash,
            amount: request.expected_amount,
            confirmations,
            vault_address: request.vault_address,
            timestamp: crate::block::current_unix_timestamp_ms(),
            oracle_signature: oracle_sig,
            oracle_pubkey,
        })
    }

    async fn verify_ltc_deposit(&mut self, request: DepositRequest) -> Result<DepositProof, String> {
        println!("[Oracle] Verifying LTC deposit: {}", request.tx_hash);

        // Verify on Litecoin blockchain via BlockCypher
        let (verified, confirmations) = self.ltc_client
            .verify_deposit(&request.tx_hash, &request.vault_address, request.expected_amount)
            .await?;

        if !verified {
            return Err("Deposit not found or insufficient amount".to_string());
        }

        // Check confirmations
        if confirmations < self.config.min_confirmations_ltc {
            return Err(format!(
                "Insufficient confirmations: {} (need {})",
                confirmations, self.config.min_confirmations_ltc
            ));
        }

        println!("[Oracle] ✓ Verified: {} confirmations", confirmations);

        // Generate Oracle signature
        let message = format!(
            "DEPOSIT:{}:{}:{}:{}:{}",
            request.chain,
            request.expected_amount,
            request.tx_hash,
            request.expected_amount,
            request.requester
        );

        let oracle_sig = self.oracle_keypair.sign_hex(message.as_bytes());
        let oracle_pubkey = self.oracle_keypair.public_key_hex();

        // Mark as processed
        self.processed_deposits.insert(request.tx_hash.clone());

        Ok(DepositProof {
            verified: true,
            tx_hash: request.tx_hash,
            amount: request.expected_amount,
            confirmations,
            vault_address: request.vault_address,
            timestamp: crate::block::current_unix_timestamp_ms(),
            oracle_signature: oracle_sig,
            oracle_pubkey,
        })
    }

    /// Bridge Function: Evaluate Betting Outcomes and Trigger Slashing
    pub async fn process_betting_outcomes(&mut self) {
        // 1. Evaluate Bets via Neural Network
        let settled = self.predictor.evaluate_and_learn(&mut self.fetcher).await;
        
        if settled.is_empty() {
            return;
        }
        
        println!("[Oracle] Processing {} settled bets for Collateral impact...", settled.len());
        
        let mut l2 = self.layer2.lock().unwrap();
        
        // 2. Iterate outcomes and Slash if needed
        for bet in settled {
            if let Some(outcome) = bet.outcome {
                if !outcome.correct {
                    // LOSS: Slash the collateral
                    // Entity is the Model/Worker ID.
                    // Assuming self.predictor.token_id is the entity if minted, or worker_id
                    let entity = self.predictor.nft_token_id.clone().unwrap_or(self.predictor.worker_id.clone());
                    
                    let slash_amount = bet.stake_amount; // Lose full stake? or portion? 
                    // Betting stake is separate from "Collateral Stake"?
                    // If Betting Stake was "Locked Collateral", then we slash it.
                    
                    // Simplified: We assume Betting Risk comes from the Main Staked Balance in L2.
                    println!("[Oracle] ⚔️ Slashing {} by {} for incorrect prediction.", entity, slash_amount);
                    
                    match l2.collateral.slash(&entity, slash_amount) {
                         Ok(slashed) => println!("[Oracle] ✅ Slashed {}. Insurance Fund increased.", slashed),
                         Err(e) => println!("[Oracle] ⚠️ Slashing failed (insufficient stake?): {}", e),
                    }
                } else {
                    // WINNING: Reward the entity
                    let entity = self.predictor.nft_token_id.clone().unwrap_or(self.predictor.worker_id.clone());
                    let reward_amount = bet.stake_amount; // winning matching stake
                    
                    println!("[Oracle] 🏆 Rewarding {} with {} COMPASS for correct prediction.", entity, reward_amount);
                    
                    // 1. Mint new tokens (Inflationary reward for intelligence)
                    l2.economics.mint_rewards(reward_amount);
                    
                    // 2. Add to stake/balance
                    l2.collateral.reward(entity, reward_amount);
                }
            }
        }
        // Save L2 state
        let _ = l2.save("layer2.json");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_oracle_verification() {
        // This would require a real testnet transaction
        // For now, just test the structure
    }
}
