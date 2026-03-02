use crate::challenge::{Challenge, ChallengeGenerator};
use crate::mml_path::MMLPathValidator;
use crate::neural_path::NeuralPathValidator;
use ai3_lib::{AI3Engine, Tensor};
use pot_o_core::{TribeError, TribeResult};
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
    pub engine: AI3Engine,
    pub challenge_gen: ChallengeGenerator,
    pub mml_validator: MMLPathValidator,
    pub neural_validator: NeuralPathValidator,
}

impl PotOConsensus {
    pub fn new(difficulty: u64, max_tensor_dim: usize) -> Self {
        Self {
            engine: AI3Engine::new(),
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

        // 2. Verify MML score meets threshold
        if !self
            .mml_validator
            .validate(proof.mml_score, challenge.mml_threshold)
        {
            return Ok(false);
        }

        // 3. Verify path distance
        if proof.path_distance > challenge.path_distance_max {
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
}
