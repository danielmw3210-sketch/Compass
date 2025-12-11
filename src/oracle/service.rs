use crate::crypto::KeyPair;
use crate::oracle::chains::{BitcoinClient, LitecoinClient};
use crate::oracle::types::{DepositProof, DepositRequest, OracleConfig};
use std::collections::HashSet;

pub struct OracleService {
    config: OracleConfig,
    btc_client: BitcoinClient,
    ltc_client: LitecoinClient,
    oracle_keypair: KeyPair,
    processed_deposits: HashSet<String>,
}

impl OracleService {
    pub fn new(config: OracleConfig, oracle_keypair: KeyPair) -> Self {
        let btc_client = BitcoinClient::new(config.blockcypher_api_key.clone());
        let ltc_client = LitecoinClient::new(config.blockcypher_api_key.clone());

        Self {
            config,
            btc_client,
            ltc_client,
            oracle_keypair,
            processed_deposits: HashSet::new(),
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
