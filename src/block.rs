use crate::crypto::KeyPair;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A full block: header + transactions
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Vec<u8>>,
}

/// Different kinds of blocks Compass can produce
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
    Transfer {
        from: String,
        to: String,
        asset: String, // "Compass" or "cLTC", "cSOL", etc.
        amount: u64,
        nonce: u64,
        fee: u64,
    },
    Mint {
        vault_id: String,
        collateral_asset: String,
        collateral_amount: u64,
        compass_asset: String,
        mint_amount: u64,
        owner: String,
        tx_proof: String, // External chain tx hash
        oracle_signature: String,
        fee: u64,
    },
    Burn {
        vault_id: String,
        compass_asset: String,
        burn_amount: u64,
        redeemer: String,
        destination_address: String, // External chain address
        fee: u64,
    },
}

impl BlockType {
    pub fn to_code(&self) -> u8 {
        match self {
            BlockType::PoH { .. } => 0,
            BlockType::Work => 1,
            BlockType::Proposal { .. } => 2,
            BlockType::Reward { .. } => 5,
            BlockType::Vote { .. } => 6,
            BlockType::Transfer { .. } => 7,
            BlockType::Mint { .. } => 8,
            BlockType::Burn { .. } => 9,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockHeader {
    pub index: u64,
    pub block_type: BlockType, // use the enum, not String
    pub proposer: String,
    pub signature_hex: String,
    pub prev_hash: String,
    pub hash: String,
    pub timestamp: u64,
}

impl BlockHeader {
    /// Calculate SHA-256 hash of block contents (exclude signature from canonical input)
    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();

        let mut data = format!(
            "{}:{}:{}:{}:{}",
            self.index,
            self.block_type.to_code(),
            self.proposer,
            self.timestamp,
            self.prev_hash,
        );

        match &self.block_type {
            BlockType::PoH {
                tick,
                iterations,
                hash,
            } => {
                data.push_str(&format!(
                    ":poh_tick={}:poh_iterations={}:poh_hash={}",
                    tick, iterations, hash
                ));
            }
            BlockType::Proposal {
                id,
                proposer,
                text,
                deadline,
            } => {
                data.push_str(&format!(
                    ":proposal_id={}:proposal_proposer={}:proposal_text={}:proposal_deadline={}",
                    id, proposer, text, deadline
                ));
            }
            BlockType::Reward {
                recipient,
                amount,
                asset,
                reason,
            } => {
                data.push_str(&format!(
                    ":reward_recipient={}:reward_amount={}:reward_asset={}:reward_reason={}",
                    recipient, amount, asset, reason
                ));
            }
            BlockType::Transfer {
                from,
                to,
                asset,
                amount,
                nonce,
                fee,
            } => {
                data.push_str(&format!(
                    ":transfer_from={}:transfer_to={}:transfer_asset={}:transfer_amount={}:transfer_nonce={}:fee={}",
                    from, to, asset, amount, nonce, fee
                ));
            }
            BlockType::Mint {
                vault_id,
                collateral_asset,
                collateral_amount,
                compass_asset,
                mint_amount,
                owner,
                tx_proof,
                oracle_signature,
                fee,
            } => {
                data.push_str(&format!(
                    ":mint_vault={}:col_asset={}:col_amt={}:comp_asset={}:mint_amt={}:owner={}:proof={}:oracle_sig={}:fee={}",
                    vault_id, collateral_asset, collateral_amount, compass_asset, mint_amount, owner, tx_proof, oracle_signature, fee
                ));
            }
            BlockType::Burn {
                vault_id,
                compass_asset,
                burn_amount,
                redeemer,
                destination_address,
                fee,
            } => {
                data.push_str(&format!(
                    ":burn_vault={}:comp_asset={}:burn_amt={}:redeemer={}:dest={}:fee={}",
                    vault_id, compass_asset, burn_amount, redeemer, destination_address, fee
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

// Proposal logic
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
    index: u64,
    admin_wallet_id: String,
    proposal_text: String,
    deadline_ms: u64,
    prev_hash: String,
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

    let mut header = BlockHeader {
        index,
        block_type: BlockType::Proposal {
            id,
            proposer: admin_wallet_id.clone(),
            text: proposal_text,
            deadline: deadline_ms,
        },
        proposer: admin_wallet_id,
        signature_hex: String::new(),
        prev_hash,
        hash: String::new(),
        timestamp: now,
    };

    let pre_sign_hash = header.calculate_hash();
    let sig_hex = sign_hex_fn(pre_sign_hash.as_bytes());
    if sig_hex.is_empty() {
        return Err(ProposalError::SigningFailed);
    }

    header.signature_hex = sig_hex;
    header.hash = header.calculate_hash();

    Ok(header)
}

/// PoH block (signed by admin)
pub fn create_poh_block(
    index: u64,
    prev_hash: String,
    tick: u64,
    iterations: u64,
    end_vdf_hash: Vec<u8>,
    admin: &KeyPair,
) -> BlockHeader {
    let timestamp = current_unix_timestamp_ms();
    let hash_hex = hex::encode(&end_vdf_hash);

    let mut header = BlockHeader {
        index,
        block_type: BlockType::PoH {
            tick,
            iterations,
            hash: hash_hex,
        },
        proposer: "admin".to_string(),
        timestamp,
        signature_hex: String::new(),
        prev_hash,
        hash: String::new(),
    };

    let pre_sign_hash = header.calculate_hash();
    let signature_hex = admin.sign_hex(pre_sign_hash.as_bytes());

    header.signature_hex = signature_hex;
    header.hash = header.calculate_hash();
    header
}

/// Vote block
pub fn create_vote_block(
    index: u64,
    voter_wallet_id: String,
    proposal_id: u64,
    choice: bool,
    prev_hash: String,
    voter: &KeyPair,
) -> BlockHeader {
    let mut header = BlockHeader {
        index,
        block_type: BlockType::Vote {
            proposal_id,
            voter: voter_wallet_id.clone(),
            choice,
        },
        proposer: voter_wallet_id,
        timestamp: current_unix_timestamp_ms(),
        signature_hex: String::new(),
        prev_hash,
        hash: String::new(),
    };

    let pre_sign_hash = header.calculate_hash();
    let sig_hex = voter.sign_hex(pre_sign_hash.as_bytes());
    header.signature_hex = sig_hex;
    header.hash = header.calculate_hash();

    header
}

/// Reward block (signed by admin)
pub fn create_reward_block(
    index: u64,
    admin_wallet_id: String,
    recipient: String,
    amount: u64,
    asset: String,
    reason: String,
    prev_hash: String,
    admin: &KeyPair,
) -> BlockHeader {
    let mut header = BlockHeader {
        index,
        block_type: BlockType::Reward {
            recipient,
            amount,
            asset,
            reason,
        },
        proposer: admin_wallet_id.clone(),
        timestamp: current_unix_timestamp_ms(),
        signature_hex: String::new(),
        prev_hash,
        hash: String::new(),
    };

    let pre_sign_hash = header.calculate_hash();
    let sig_hex = admin.sign_hex(pre_sign_hash.as_bytes());
    header.signature_hex = sig_hex;
    header.hash = header.calculate_hash();

    header
}

/// Transfer block (signed by sender)
pub fn create_transfer_block(
    index: u64,
    from: String,
    to: String,
    asset: String,
    amount: u64,
    nonce: u64,
    fee: u64,
    prev_hash: String,
    sender_keypair: &KeyPair,
) -> BlockHeader {
    let mut header = BlockHeader {
        index,
        block_type: BlockType::Transfer {
            from: from.clone(),
            to,
            asset,
            amount,
            nonce,
            fee,
        },
        proposer: from,
        timestamp: current_unix_timestamp_ms(),
        signature_hex: String::new(),
        prev_hash,
        hash: String::new(),
    };

    let pre_sign_hash = header.calculate_hash();
    let sig_hex = sender_keypair.sign_hex(pre_sign_hash.as_bytes());
    header.signature_hex = sig_hex;
    header.hash = header.calculate_hash();

    header
}
