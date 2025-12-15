use crate::crypto::KeyPair;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use crate::encoding::CanonicalSerialize;
use std::io::Write;
use tracing::{debug, error};

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
        proof: String, // Wesolowski proof
    },
    Genesis,
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
        collateral_asset: String,
        compass_asset: String,
        burn_amount: u64,
        redeemer: String,
        destination_address: String, // External chain address
        fee: u64,
    },
    ValidatorRegistration {
        validator_id: String,
        pubkey: String,
        stake_amount: u64,
        signature: String,
    },
}

impl CanonicalSerialize for BlockType {
    fn canonical_serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            BlockType::PoH { tick, iterations, hash, proof } => {
                0u8.canonical_serialize(writer)?;
                tick.canonical_serialize(writer)?;
                iterations.canonical_serialize(writer)?;
                hash.canonical_serialize(writer)?;
                proof.canonical_serialize(writer)?;
            }
            BlockType::Genesis => { 255u8.canonical_serialize(writer)?; }
            BlockType::Work => { 1u8.canonical_serialize(writer)?; }
            BlockType::Proposal { id, proposer, text, deadline } => {
                2u8.canonical_serialize(writer)?;
                id.canonical_serialize(writer)?;
                proposer.canonical_serialize(writer)?;
                text.canonical_serialize(writer)?;
                deadline.canonical_serialize(writer)?;
            }
            BlockType::Reward { recipient, amount, asset, reason } => {
                5u8.canonical_serialize(writer)?;
                recipient.canonical_serialize(writer)?;
                amount.canonical_serialize(writer)?;
                asset.canonical_serialize(writer)?;
                reason.canonical_serialize(writer)?;
            },
            BlockType::Vote { proposal_id, voter, choice } => {
                6u8.canonical_serialize(writer)?;
                proposal_id.canonical_serialize(writer)?;
                voter.canonical_serialize(writer)?;
                choice.canonical_serialize(writer)?;
            },
            BlockType::Transfer { from, to, asset, amount, nonce, fee } => {
                7u8.canonical_serialize(writer)?;
                from.canonical_serialize(writer)?;
                to.canonical_serialize(writer)?;
                asset.canonical_serialize(writer)?;
                amount.canonical_serialize(writer)?;
                nonce.canonical_serialize(writer)?;
                fee.canonical_serialize(writer)?;
            },
            BlockType::Mint { vault_id, collateral_asset, collateral_amount, compass_asset, mint_amount, owner, tx_proof, oracle_signature, fee } => {
                8u8.canonical_serialize(writer)?;
                vault_id.canonical_serialize(writer)?;
                collateral_asset.canonical_serialize(writer)?;
                collateral_amount.canonical_serialize(writer)?;
                compass_asset.canonical_serialize(writer)?;
                mint_amount.canonical_serialize(writer)?;
                owner.canonical_serialize(writer)?;
                tx_proof.canonical_serialize(writer)?;
                oracle_signature.canonical_serialize(writer)?;
                fee.canonical_serialize(writer)?;
            },
            BlockType::Burn { vault_id, collateral_asset, compass_asset, burn_amount, redeemer, destination_address, fee } => {
                9u8.canonical_serialize(writer)?;
                vault_id.canonical_serialize(writer)?;
                collateral_asset.canonical_serialize(writer)?;
                compass_asset.canonical_serialize(writer)?;
                burn_amount.canonical_serialize(writer)?;
                redeemer.canonical_serialize(writer)?;
                destination_address.canonical_serialize(writer)?;
                fee.canonical_serialize(writer)?;
            },
            BlockType::ValidatorRegistration { validator_id, pubkey, stake_amount, signature } => {
                10u8.canonical_serialize(writer)?;
                validator_id.canonical_serialize(writer)?;
                pubkey.canonical_serialize(writer)?;
                stake_amount.canonical_serialize(writer)?;
                signature.canonical_serialize(writer)?;
            }
        }
        Ok(())
    }
}

impl BlockType {
    pub fn to_code(&self) -> u8 {
        match self {
            BlockType::PoH { .. } => 0,
            BlockType::Genesis => 255,
            BlockType::Work => 1,
            BlockType::Proposal { .. } => 2,
            BlockType::Reward { .. } => 5,
            BlockType::Vote { .. } => 6,
            BlockType::Transfer { .. } => 7,
            BlockType::Mint { .. } => 8,
            BlockType::Burn { .. } => 9,
            BlockType::ValidatorRegistration { .. } => 10,
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
    /// Serializes the header fields (excluding signature/hash) into a deterministic binary format
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, crate::error::CompassError> {
        // Production Grade: Using manual strict serialization
        let mut buf = Vec::new();
        
        // Follow tuple order or struct order? Strict Order:
        // Index, BlockType, Proposer, Timestamp, PrevHash
        // (Signature and SelfHash are excluded)

        self.index.canonical_serialize(&mut buf).map_err(|e| crate::error::CompassError::SerializationError(e.to_string()))?;
        self.block_type.canonical_serialize(&mut buf).map_err(|e| crate::error::CompassError::SerializationError(e.to_string()))?;
        self.proposer.canonical_serialize(&mut buf).map_err(|e| crate::error::CompassError::SerializationError(e.to_string()))?;
        self.timestamp.canonical_serialize(&mut buf).map_err(|e| crate::error::CompassError::SerializationError(e.to_string()))?;
        self.prev_hash.canonical_serialize(&mut buf).map_err(|e| crate::error::CompassError::SerializationError(e.to_string()))?;
        
        Ok(buf)
    }

    /// Calculate SHA-256 hash of block contents (exclude signature from canonical input)
    pub fn calculate_hash(&self) -> Result<String, crate::error::CompassError> {
        let mut hasher = Sha256::new();
        // Use our new strict serializer
        hasher.update(&self.canonical_bytes()?);
        Ok(hex::encode(hasher.finalize()))
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
) -> Result<BlockHeader, crate::error::CompassError> {
    if !is_admin(&admin_wallet_id) {
        return Err(crate::error::CompassError::InvalidState("Not Admin".to_string()));
    }
    if proposal_text.trim().is_empty() {
        return Err(crate::error::CompassError::InvalidState("Empty Text".to_string()));
    }
    let now = current_unix_timestamp_ms();
    if deadline_ms <= now {
        return Err(crate::error::CompassError::InvalidState("Deadline in past".to_string()));
    }

    let mut id = id_gen_fn();
    if id == 0 {
        id = now;
    }
    if id_exists_fn(id) {
        return Err(crate::error::CompassError::InvalidState("Id collision".to_string()));
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

    let pre_sign_hash = header.calculate_hash()?;
    let sig_hex = sign_hex_fn(pre_sign_hash.as_bytes());
    if sig_hex.is_empty() {
        return Err(crate::error::CompassError::InvalidSignature);
    }

    header.signature_hex = sig_hex;
    header.hash = header.calculate_hash()?;

    Ok(header)
}

/// PoH block (signed by admin)
pub fn create_poh_block(
    index: u64,
    prev_hash: String,
    tick: u64,
    iterations: u64,
    end_vdf_hash: Vec<u8>,
    proof_bytes: Vec<u8>,
    timestamp: u64, // Explicit timestamp
    admin: &KeyPair,
) -> Result<BlockHeader, crate::error::CompassError> {
    // timestamp is passed in
    let hash_hex = hex::encode(&end_vdf_hash);
    let proof_hex = hex::encode(&proof_bytes);

    let mut header = BlockHeader {
        index,
        block_type: BlockType::PoH {
            tick,
            iterations,
            hash: hash_hex,
            proof: proof_hex,
        },
        proposer: "admin".to_string(),
        timestamp,
        signature_hex: String::new(),
        prev_hash,
        hash: String::new(),
    };

    let pre_sign_hash = header.calculate_hash()?;
    
    // Decode to raw bytes for signing (safer/cleaner than signing ASCII hex)
    let raw_hash = hex::decode(&pre_sign_hash).map_err(|e| crate::error::CompassError::SerializationError(e.to_string()))?;

    // Sign the RAW bytes
    let signature_hex = admin.sign_hex(&raw_hash);

    if index == 1 {
         debug!("DEBUG[Node]: Block 1 PreSign Hash: {}", pre_sign_hash);
         debug!("DEBUG[Node]: Block 1 Sig: {}", signature_hex);
         
         // Verify immediately with raw bytes
            let pk_bytes = admin.public_key();
            let pk_hex = hex::encode(pk_bytes.as_bytes());
            
            if !crate::crypto::verify_with_pubkey_hex(&raw_hash, &signature_hex, &pk_hex) {
                // This is a sanity check panic in development, converting to error log + panic for now as it's critical
                error!("Static Test FAILED (Signature is Invalid)");
                panic!("Static Test FAILED (Signature is Invalid)");
            } else {
                debug!("DEBUG[Node]: Block 1 Self-Verify PASSED.");
            }
    }

    header.signature_hex = signature_hex;
    header.hash = header.calculate_hash()?;
    Ok(header)
}

/// Vote block
pub fn create_vote_block(
    index: u64,
    voter_wallet_id: String,
    proposal_id: u64,
    choice: bool,
    prev_hash: String,
    voter: &KeyPair,
) -> Result<BlockHeader, crate::error::CompassError> {
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

    let pre_sign_hash = header.calculate_hash()?;
    let sig_hex = voter.sign_hex(pre_sign_hash.as_bytes());
    header.signature_hex = sig_hex;
    header.hash = header.calculate_hash()?;

    Ok(header)
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
) -> Result<BlockHeader, crate::error::CompassError> {
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

    let pre_sign_hash = header.calculate_hash()?;
    let sig_hex = admin.sign_hex(pre_sign_hash.as_bytes());
    header.signature_hex = sig_hex;
    header.hash = header.calculate_hash()?;

    Ok(header)
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
) -> Result<BlockHeader, crate::error::CompassError> {
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

    let pre_sign_hash = header.calculate_hash()?;
    let sig_hex = sender_keypair.sign_hex(pre_sign_hash.as_bytes());
    header.signature_hex = sig_hex;
    header.hash = header.calculate_hash()?;

    Ok(header)
}
