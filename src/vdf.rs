#![allow(dead_code)]
use num_bigint::BigUint;
use num_traits::{One, Zero, ToPrimitive};
use num_integer::Integer;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug)]
pub struct WesolowskiVDF {
    pub modulus: BigUint,
}

// 2048-bit modulus (Pseudo-random for Alpha - DO NOT USE IN PRODUCTION)
pub const ALPHA_MODULUS: &[u8] = b"\
    F2B487A29C8B52E1097C0214358899A1C5243166827725920D22588358485292\
    A137458237582375928375923847592384759102938475102938475129384751\
    1234123412341234123412341234123412341234123412341234123412341234\
    FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF\
    F2B487A29C8B52E1097C0214358899A1C5243166827725920D22588358485292\
    A137458237582375928375923847592384759102938475102938475129384751\
    1234123412341234123412341234123412341234123412341234123412341234\
    FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF\
";

impl WesolowskiVDF {
    pub fn new(modulus_bytes: &[u8]) -> Self {
        WesolowskiVDF {
            modulus: BigUint::from_bytes_be(modulus_bytes),
        }
    }

    /// Run VDF (Squaring only). Returns y.
    pub fn eval(&self, input: &[u8], iterations: u64) -> Vec<u8> {
        let g = self.hash_to_group(input);
        let mut y = g;
        for _ in 0..iterations {
            y = y.modpow(&BigUint::from(2u32), &self.modulus);
        }
        y.to_bytes_be()
    }

    /// Run the VDF for T iterations. 
    /// Returns (y, proof) where y = g^(2^T) mod N
    pub fn solve(&self, input: &[u8], iterations: u64) -> (Vec<u8>, Vec<u8>) {
        let g = self.hash_to_group(input);
        
        // 1. Compute y = g^(2^T)
        // We do this by squaring T times.
        let mut y = g.clone();
        for _ in 0..iterations {
            y = y.modpow(&BigUint::from(2u32), &self.modulus);
        }

        // 2. Generate Challenge l
        let l = self.hash_prime(&g, &y);

        // 3. Compute Proof pi = g^Q mod N where Q = floor(2^T / l)
        // Calculate 2^T first. 
        // Note: 2^T can be very large.
        let exponent = BigUint::from(2u32).pow(iterations as u32);
        let q = &exponent / &l;
        
        let pi = g.modpow(&q, &self.modulus);

        (y.to_bytes_be(), pi.to_bytes_be())
    }

    /// Verify the proof
    /// Check: pi^l * g^r == y (mod N)
    /// where r = 2^T mod l
    pub fn verify(&self, input: &[u8], iterations: u64, result: &[u8], proof: &[u8]) -> bool {
        let g = self.hash_to_group(input);
        let y = BigUint::from_bytes_be(result);
        let pi = BigUint::from_bytes_be(proof);
        
        let l = self.hash_prime(&g, &y);
        
        // r = 2^T mod l
        // We can compute this efficiently using modular exponentiation on l
        let r = BigUint::from(2u32).modpow(&BigUint::from(iterations), &l);
        
        // lhs = pi^l * g^r
        let lhs = (pi.modpow(&l, &self.modulus) * g.modpow(&r, &self.modulus)) % &self.modulus;
        
        lhs == y
    }

    // Helper: Map input bytes to a group element (simple hash mod N)
    fn hash_to_group(&self, input: &[u8]) -> BigUint {
        let mut hasher = Sha256::new();
        hasher.update(input);
        let h = hasher.finalize();
        BigUint::from_bytes_be(&h) % &self.modulus
    }

    // Helper: Generate prime challenge l = H(g, y)
    fn hash_prime(&self, g: &BigUint, y: &BigUint) -> BigUint {
        let mut hasher = Sha256::new();
        hasher.update(&g.to_bytes_be());
        hasher.update(&y.to_bytes_be());
        let h = hasher.finalize();
        let mut l = BigUint::from_bytes_be(&h);
        
        // Ensure odd
        if l.is_even() {
            l += BigUint::one();
        }
        // TODO: Ensure prime (Miller-Rabin). 
        // For alpha, just ensuring odd/hash derivation is "good enough" for non-adversarial.
        l
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wesolowski_vdf_correctness() {
        let vdf = WesolowskiVDF::new(ALPHA_MODULUS);
        let input = b"CompassVDFTestSeed";
        let iterations = 20; // Enough to test modular exponentiation logic

        // 1. Solve
        let (y, proof) = vdf.solve(input, iterations);
        
        // 2. Verify Correct Proof
        let valid = vdf.verify(input, iterations, &y, &proof);
        assert!(valid, "Proof should be valid");

        // 3. Verify Invalid Proof (tampered proof)
        let mut bad_proof = proof.clone();
        if bad_proof.len() > 0 {
            if bad_proof[0] < 255 { bad_proof[0] += 1; } else { bad_proof[0] -= 1; }
            let valid_bad = vdf.verify(input, iterations, &y, &bad_proof);
            assert!(!valid_bad, "Tampered proof should be invalid");
        }

        // 4. Verify Eval matches Solve result
        let eval_y = vdf.eval(input, iterations);
        assert_eq!(y, eval_y, "Eval result should match Solve result");
    }
}
