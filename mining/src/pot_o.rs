//! PoT-O consensus: proof generation, verification, and engine stats.

use crate::challenge::{Challenge, ChallengeGenerator};
use crate::mml_path::MMLPathValidator;
use crate::neural_path::NeuralPathValidator;
use ai3_lib::{AI3Engine, EngineStats, TensorEngine};
use pot_o_core::TribeResult;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Instant;

/// The full PoT-O proof submitted by a miner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotOProof {
    pub challenge_id: String,
    pub challenge_hash: String,
    pub tensor_result_hash: String,
    pub mml_score: f64,
    pub path_signature: String,
    pub path_distance: u32,
    pub computation_nonce: u64,
    pub computation_hash: String,
    pub miner_pubkey: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Payload sent to the on-chain program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofPayload {
    pub proof: PotOProof,
    pub signature: Vec<u8>,
}

/// The PoT-O consensus engine. Orchestrates challenge generation,
/// tensor computation, MML validation, and neural path matching.
pub struct PotOConsensus {
    pub engine: Box<dyn TensorEngine>,
    pub challenge_gen: ChallengeGenerator,
    pub mml_validator: MMLPathValidator,
    pub neural_validator: NeuralPathValidator,
}

impl PotOConsensus {
    pub fn new(difficulty: u64, max_tensor_dim: usize) -> Self {
        Self {
            engine: Box::new(AI3Engine::new()),
            challenge_gen: ChallengeGenerator::new(difficulty, max_tensor_dim),
            mml_validator: MMLPathValidator::default(),
            neural_validator: NeuralPathValidator::default(),
        }
    }

    /// Generate a new challenge from the latest Solana slot data.
    pub fn generate_challenge(&self, slot: u64, slot_hash: &str) -> TribeResult<Challenge> {
        self.challenge_gen.generate(slot, slot_hash)
    }

    /// Attempt to mine a proof for a given challenge. Iterates nonces until
    /// both MML and neural-path constraints are satisfied, or max_iterations is hit.
    pub fn mine(
        &self,
        challenge: &Challenge,
        miner_pubkey: &str,
        max_iterations: u64,
    ) -> TribeResult<Option<PotOProof>> {
        let start = Instant::now();
        let task = challenge.to_mining_task(miner_pubkey);

        let output_tensor = self.engine.execute_task(&task)?;
        let mml_score = self
            .mml_validator
            .compute_mml_score(&challenge.input_tensor, &output_tensor)?;

        for nonce in 0..max_iterations {
            let actual_path = self
                .neural_validator
                .compute_actual_path(&output_tensor, nonce)?;
            let expected = self.neural_validator.expected_path_signature(&challenge.id);
            let min_len = actual_path.len().min(expected.len());
            let distance = NeuralPathValidator::hamming_distance(
                &actual_path[..min_len],
                &expected[..min_len],
            );

            if distance <= challenge.path_distance_max
                && self
                    .mml_validator
                    .validate(mml_score, challenge.mml_threshold)
            {
                let tensor_result_hash = output_tensor.calculate_hash();
                let path_sig = NeuralPathValidator::path_to_hex(&actual_path);
                let computation_hash = Self::compute_proof_hash(
                    &challenge.id,
                    &tensor_result_hash,
                    mml_score,
                    &path_sig,
                    nonce,
                );

                let elapsed = start.elapsed();
                self.engine.record_result(true, elapsed);

                return Ok(Some(PotOProof {
                    challenge_id: challenge.id.clone(),
                    challenge_hash: challenge.slot_hash.clone(),
                    tensor_result_hash,
                    mml_score,
                    path_signature: path_sig,
                    path_distance: distance,
                    computation_nonce: nonce,
                    computation_hash,
                    miner_pubkey: miner_pubkey.to_string(),
                    timestamp: chrono::Utc::now(),
                }));
            }
        }

        self.engine.record_result(false, start.elapsed());
        Ok(None)
    }

    /// Verify a proof offline (same checks the on-chain program performs).
    /// Re-executes the tensor computation and recomputes all derived values.
    pub fn verify_proof(&self, proof: &PotOProof, challenge: &Challenge) -> TribeResult<bool> {
        // 1. Verify computation hash integrity
        let expected_hash = Self::compute_proof_hash(
            &proof.challenge_id,
            &proof.tensor_result_hash,
            proof.mml_score,
            &proof.path_signature,
            proof.computation_nonce,
        );
        if expected_hash != proof.computation_hash {
            return Ok(false);
        }

        // 2. Sanity: MML score must be non-negative
        if proof.mml_score < 0.0 {
            return Ok(false);
        }

        // 3. Re-execute the tensor operation and verify the result hash
        let task = challenge.to_mining_task(&proof.miner_pubkey);
        let output_tensor = self.engine.execute_task(&task)?;
        let recomputed_hash = output_tensor.calculate_hash();
        if recomputed_hash != proof.tensor_result_hash {
            return Ok(false);
        }

        // 4. Recompute MML score and verify
        let recomputed_mml = self
            .mml_validator
            .compute_mml_score(&challenge.input_tensor, &output_tensor)?;
        if (recomputed_mml - proof.mml_score).abs() > f64::EPSILON {
            return Ok(false);
        }

        // 5. Verify MML score meets threshold
        if !self
            .mml_validator
            .validate(proof.mml_score, challenge.mml_threshold)
        {
            return Ok(false);
        }

        // 6. Recompute neural path and verify path_distance
        let actual_path = self
            .neural_validator
            .compute_actual_path(&output_tensor, proof.computation_nonce)?;
        let expected_path = self.neural_validator.expected_path_signature(&challenge.id);
        let min_len = actual_path.len().min(expected_path.len());
        let recomputed_distance = NeuralPathValidator::hamming_distance(
            &actual_path[..min_len],
            &expected_path[..min_len],
        );
        if recomputed_distance != proof.path_distance {
            return Ok(false);
        }

        // 7. Verify path distance within limit
        if proof.path_distance > challenge.path_distance_max {
            return Ok(false);
        }

        // 8. Verify path signature hex matches recomputed path
        let recomputed_path_sig = NeuralPathValidator::path_to_hex(&actual_path);
        if recomputed_path_sig != proof.path_signature {
            return Ok(false);
        }

        Ok(true)
    }

    /// Expected path and calc counts for this challenge (for status dashboard treemap).
    /// - expected_paths: length of the neural path signature (deterministic per challenge).
    /// - expected_calcs: 1 + difficulty (one base tensor op plus difficulty-derived steps).
    pub fn expected_paths_and_calcs(&self, challenge: &Challenge) -> (u64, u64) {
        let expected_paths = self
            .neural_validator
            .expected_path_signature(&challenge.id)
            .len() as u64;
        let expected_calcs = 1 + challenge.difficulty;
        (expected_paths, expected_calcs)
    }

    /// Expose a read-only view of engine stats via the TensorEngine abstraction.
    pub fn engine_stats(&self) -> EngineStats {
        self.engine.get_stats()
    }

    /// Compute the deterministic proof hash: sha256(challenge_id || tensor_hash || mml_score || path_sig || nonce)
    pub fn compute_proof_hash(
        challenge_id: &str,
        tensor_result_hash: &str,
        mml_score: f64,
        path_signature: &str,
        nonce: u64,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(challenge_id.as_bytes());
        hasher.update(tensor_result_hash.as_bytes());
        hasher.update(mml_score.to_le_bytes());
        hasher.update(path_signature.as_bytes());
        hasher.update(nonce.to_le_bytes());
        hex::encode(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_lifecycle() {
        let consensus = PotOConsensus::new(1, 8);
        let hash = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let challenge = consensus.generate_challenge(100, hash).unwrap();
        assert!(!challenge.id.is_empty());

        // With low difficulty and small tensors, mining should find a proof quickly
        let result = consensus
            .mine(&challenge, "test_miner_pubkey", 1000)
            .unwrap();
        assert!(result.is_some(), "Should find a proof with low difficulty");

        let proof = result.unwrap();
        let valid = consensus.verify_proof(&proof, &challenge).unwrap();
        assert!(valid, "Mined proof should verify");
    }

    #[test]
    fn test_proof_hash_deterministic() {
        let h1 = PotOConsensus::compute_proof_hash("chal", "tensor", 0.5, "path", 42);
        let h2 = PotOConsensus::compute_proof_hash("chal", "tensor", 0.5, "path", 42);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_expected_paths_and_calcs() {
        let consensus = PotOConsensus::new(2, 8);
        let hash = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let challenge = consensus.generate_challenge(100, hash).unwrap();
        let (expected_paths, expected_calcs) = consensus.expected_paths_and_calcs(&challenge);
        assert!(expected_paths > 0, "expected_paths should be positive");
        assert!(expected_calcs > 0, "expected_calcs should be positive");
        let path_len = consensus
            .neural_validator
            .expected_path_signature(&challenge.id)
            .len() as u64;
        assert_eq!(
            expected_paths, path_len,
            "expected_paths should match path signature length"
        );
        assert_eq!(expected_calcs, 1 + challenge.difficulty);
    }

    #[test]
    fn test_tampered_tensor_result_hash_rejected() {
        let consensus = PotOConsensus::new(1, 8);
        let hash = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let challenge = consensus.generate_challenge(100, hash).unwrap();
        let mut proof = consensus
            .mine(&challenge, "test_miner_pubkey", 1000)
            .unwrap()
            .expect("should mine");

        proof.tensor_result_hash = "deadbeef".repeat(8);
        proof.computation_hash = PotOConsensus::compute_proof_hash(
            &proof.challenge_id,
            &proof.tensor_result_hash,
            proof.mml_score,
            &proof.path_signature,
            proof.computation_nonce,
        );

        let valid = consensus.verify_proof(&proof, &challenge).unwrap();
        assert!(!valid, "Tampered tensor_result_hash must be rejected");
    }

    #[test]
    fn test_negative_mml_score_rejected() {
        let consensus = PotOConsensus::new(1, 8);
        let hash = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let challenge = consensus.generate_challenge(100, hash).unwrap();
        let mut proof = consensus
            .mine(&challenge, "test_miner_pubkey", 1000)
            .unwrap()
            .expect("should mine");

        proof.mml_score = -1.0;
        proof.computation_hash = PotOConsensus::compute_proof_hash(
            &proof.challenge_id,
            &proof.tensor_result_hash,
            proof.mml_score,
            &proof.path_signature,
            proof.computation_nonce,
        );

        let valid = consensus.verify_proof(&proof, &challenge).unwrap();
        assert!(!valid, "Negative MML score must be rejected");
    }

    #[test]
    fn test_tampered_mml_score_rejected() {
        let consensus = PotOConsensus::new(1, 8);
        let hash = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let challenge = consensus.generate_challenge(100, hash).unwrap();
        let mut proof = consensus
            .mine(&challenge, "test_miner_pubkey", 1000)
            .unwrap()
            .expect("should mine");

        proof.mml_score = 999.0;
        proof.computation_hash = PotOConsensus::compute_proof_hash(
            &proof.challenge_id,
            &proof.tensor_result_hash,
            proof.mml_score,
            &proof.path_signature,
            proof.computation_nonce,
        );

        let valid = consensus.verify_proof(&proof, &challenge).unwrap();
        assert!(!valid, "Tampered MML score must be rejected");
    }

    #[test]
    fn test_tampered_path_distance_rejected() {
        let consensus = PotOConsensus::new(1, 8);
        let hash = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let challenge = consensus.generate_challenge(100, hash).unwrap();
        let mut proof = consensus
            .mine(&challenge, "test_miner_pubkey", 1000)
            .unwrap()
            .expect("should mine");

        proof.path_distance = proof.path_distance.wrapping_add(100);

        let valid = consensus.verify_proof(&proof, &challenge).unwrap();
        assert!(!valid, "Tampered path_distance must be rejected");
    }

    #[test]
    fn test_tampered_path_signature_rejected() {
        let consensus = PotOConsensus::new(1, 8);
        let hash = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let challenge = consensus.generate_challenge(100, hash).unwrap();
        let mut proof = consensus
            .mine(&challenge, "test_miner_pubkey", 1000)
            .unwrap()
            .expect("should mine");

        proof.path_signature = "ff".repeat(64);
        proof.computation_hash = PotOConsensus::compute_proof_hash(
            &proof.challenge_id,
            &proof.tensor_result_hash,
            proof.mml_score,
            &proof.path_signature,
            proof.computation_nonce,
        );

        let valid = consensus.verify_proof(&proof, &challenge).unwrap();
        assert!(!valid, "Tampered path_signature must be rejected");
    }
}
