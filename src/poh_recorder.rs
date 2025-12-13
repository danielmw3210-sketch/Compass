use std::sync::{Arc, Mutex};
use sha2::{Sha256, Digest};
use std::time::Instant;
use crate::vdf::{WesolowskiVDF, ALPHA_MODULUS};

pub struct PoHRecorder {
    pub tick_height: u64,
    pub current_hash: Vec<u8>,
    pub hashes_per_tick: u64,
    pub vdf: WesolowskiVDF,
}

impl PoHRecorder {
    pub fn new(initial_hash: Vec<u8>, hashes_per_tick: u64) -> Self {
        PoHRecorder {
            tick_height: 0,
            current_hash: initial_hash,
            hashes_per_tick,
            vdf: WesolowskiVDF::new(ALPHA_MODULUS),
        }
    }

    pub fn restore(tick: u64, hash: Vec<u8>, hashes_per_tick: u64) -> Self {
        PoHRecorder {
            tick_height: tick,
            current_hash: hash,
            hashes_per_tick,
            vdf: WesolowskiVDF::new(ALPHA_MODULUS),
        }
    }

    // Run the VDF for one tick. Returns (start_hash, end_hash)
    // Now uses Wesolowski VDF (Squaring)
    pub fn tick(&mut self) -> (Vec<u8>, Vec<u8>) {
        let start_hash = self.current_hash.clone();
        
        // Eval VDF (Squaring)
        let end_hash = self.vdf.eval(&start_hash, self.hashes_per_tick);

        self.current_hash = end_hash.clone();
        self.tick_height += 1;

        (start_hash, end_hash)
    }

    /// Prove a specific transition.
    pub fn prove(&self, start_hash: &[u8], iterations: u64) -> Vec<u8> {
        self.vdf.solve(start_hash, iterations).1
    }

    /// Verify a specific transition.
    pub fn verify(start_hash: &[u8], end_hash: &[u8], iterations: u64, proof: &[u8]) -> bool {
        let vdf = WesolowskiVDF::new(ALPHA_MODULUS);
        vdf.verify(start_hash, iterations, end_hash, proof)
    }
}
