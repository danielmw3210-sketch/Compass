use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use crate::crypto::KeyPair;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BlockType {
    PoH {
        tick: u64,
        iterations: u64,
        hash: String, // end_hash of the VDF
    },
    Work,
    Proposal {
        id: u64,
        proposer: String,
        text: String,
        deadline: u64,
    },
    Reward {
        recipient: String,
        amount: u64,
        asset: String,
        reason: String,
    },
    Vote {
        proposal_id: u64,
        voter: String,
        choice: bool, // yes/no
    },
    // Future: FinanceAnchor ...
}

impl BlockType {
    pub fn to_code(&self) -> u8 {
        match self {
            BlockType::PoH { .. } => 0,
            BlockType::Work => 1,
            BlockType::Proposal { .. } => 2,
            BlockType::Reward { .. } => 5,
            BlockType::Vote { .. } => 6,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockHeader {
    pub index: u64, // Added index field
    pub block_type: BlockType,
    pub proposer: String,
    pub timestamp: u64,
    pub signature_hex: Option<String>,
    pub prev_hash: Option<String>,
    pub hash: Option<String>,
}

impl BlockHeader {
    /// Calculate SHA-256 hash of block contents (exclude signature from canonical input)
    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();

        let mut data = format!(
            "{}:{}:{}:{}:{}",
            self.index, // Added index to hash
            self.block_type.to_code(),
            self.proposer,
            self.timestamp,
            self.prev_hash.clone().unwrap_or_default()
        );

        match &self.block_type {
            BlockType::PoH { tick, iterations, hash } => {
                data.push_str(&format!(
                    ":poh_tick={}:poh_iterations={}:poh_hash={}",
                    tick, iterations, hash
                ));
            }
            BlockType::Proposal { id, proposer, text, deadline } => {
                data.push_str(&format!(
                    ":proposal_id={}:proposal_proposer={}:proposal_text={}:proposal_deadline={}",
                    id, proposer, text, deadline
                ));
            }
            BlockType::Reward { recipient, amount, asset, reason } => {
                data.push_str(&format!(
                    ":reward_recipient={}:reward_amount={}:reward_asset={}:reward_reason={}",
                    recipient, amount, asset, reason
                ));
            }
            _ => {}
        }

        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    }
}

/// Admin allowlist and time helpers
pub fn is_admin(wallet_id: &str) -> bool {
    matches!(wallet_id, "admin" | "foundation" | "governance_multisig")
}

pub fn current_unix_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as u64
}

/// Proposal logic
#[derive(Debug)]
pub enum ProposalError {
    NotAdmin,
    EmptyText,
    DeadlineInPast,
    IdCollision,
    SigningFailed,
}
pub type ProposalResult<T> = Result<T, ProposalError>;

pub fn create_proposal_block(
    admin_wallet_id: String,
    proposal_text: String,
    deadline_ms: u64,
    prev_hash: Option<String>,
    sign_hex_fn: impl Fn(&[u8]) -> String,
    id_gen_fn: impl Fn() -> u64,
    id_exists_fn: impl Fn(u64) -> bool,
) -> ProposalResult<BlockHeader> {
    if !is_admin(&admin_wallet_id) {
        return Err(ProposalError::NotAdmin);
    }
    if proposal_text.trim().is_empty() {
        return Err(ProposalError::EmptyText);
    }
    let now = current_unix_timestamp_ms();
    if deadline_ms <= now {
        return Err(ProposalError::DeadlineInPast);
    }

    let mut id = id_gen_fn();
    if id == 0 {
        id = now;
    }
    if id_exists_fn(id) {
        return Err(ProposalError::IdCollision);
    }

    let proposal_variant = BlockType::Proposal {
        id,
        proposer: admin_wallet_id.clone(),
        text: proposal_text.clone(),
        deadline: deadline_ms,
    };

    let mut header = BlockHeader {
        index: 0, // Placeholder, needs to be set by Chain
        block_type: proposal_variant,
        proposer: admin_wallet_id,
        timestamp: now,
        signature_hex: None,
        prev_hash,
        hash: None,
    };

    let pre_sign_hash = header.calculate_hash();
    let sig_hex = sign_hex_fn(pre_sign_hash.as_bytes());
    if sig_hex.is_empty() {
        return Err(ProposalError::SigningFailed);
    }
    header.signature_hex = Some(sig_hex);
    header.hash = Some(header.calculate_hash());

    Ok(header)
}

/// PoH block (signed by admin)
pub fn create_poh_block(
    prev_hash: String,
    tick: u64,
    iterations: u64,
    end_vdf_hash: Vec<u8>,
    admin: &KeyPair,
) -> BlockHeader {
    let timestamp = current_unix_timestamp_ms();
    let hash_hex = hex::encode(&end_vdf_hash);
    let message = format!("PoH:{}:{}:{}:{}", tick, prev_hash, timestamp, hash_hex);
    let signature_hex = admin.sign_hex(message.as_bytes());

    let mut header = BlockHeader {
        index: 0, // Placeholder
        block_type: BlockType::PoH {
            tick,
            iterations,
            hash: hash_hex,
        },
        proposer: "admin".to_string(),
        timestamp,
        signature_hex: Some(signature_hex),
        prev_hash: Some(prev_hash),
        hash: None,
    };

    header.hash = Some(header.calculate_hash());
    header
}

/// Vote block
pub fn create_vote_block(
    voter_wallet_id: String,
    proposal_id: u64,
    choice: bool,
    prev_hash: Option<String>,
    voter: &KeyPair,
) -> BlockHeader {
    let mut header = BlockHeader {
        index: 0, // Placeholder
        block_type: BlockType::Vote {
            proposal_id,
            voter: voter_wallet_id.clone(),
            choice,
        },
        proposer: voter_wallet_id,
        timestamp: current_unix_timestamp_ms(),
        signature_hex: None,
        prev_hash,
        hash: None,
    };

    let pre_sign_hash = header.calculate_hash();
    let sig_hex = voter.sign_hex(pre_sign_hash.as_bytes());
    header.signature_hex = Some(sig_hex);
    header.hash = Some(header.calculate_hash());

    header
}

/// Reward block (signed by admin)
pub fn create_reward_block(
    admin_wallet_id: String,
    recipient: String,
    amount: u64,
    asset: String,
    reason: String,
    prev_hash: Option<String>,
    admin: &KeyPair,
) -> BlockHeader {
    let mut header = BlockHeader {
        index: 0, // Placeholder
        block_type: BlockType::Reward {
            recipient,
            amount,
            asset,
            reason,
        },
        proposer: admin_wallet_id.clone(),
        timestamp: current_unix_timestamp_ms(),
        signature_hex: None,
        prev_hash,
        hash: None,
    };

    let pre_sign_hash = header.calculate_hash();
    let sig_hex = admin.sign_hex(pre_sign_hash.as_bytes());
    header.signature_hex = Some(sig_hex);
    header.hash = Some(header.calculate_hash());

    header
}