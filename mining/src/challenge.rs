//! Challenge generation from slot/slot_hash and conversion to mining tasks.

use crate::neural_path::NeuralPathValidator;
use ai3_lib::tensor::{Tensor, TensorData, TensorShape};
use ai3_lib::MiningTask;
use pot_o_core::TribeResult;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A PoT-O mining challenge derived from a Solana slot hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    /// Unique challenge id.
    pub id: String,
    /// Solana slot number.
    pub slot: u64,
    /// Slot hash (e.g. 64-char hex).
    pub slot_hash: String,
    /// Tensor operation type (e.g. matrix_multiply, relu).
    pub operation_type: String,
    /// Input tensor for the operation.
    pub input_tensor: Tensor,
    /// Mining difficulty (path distance, etc.).
    pub difficulty: u64,
    /// MML score threshold to accept a proof.
    pub mml_threshold: f64,
    /// Maximum allowed Hamming distance for neural path.
    pub path_distance_max: u32,
    /// Max tensor dimension.
    pub max_tensor_dim: usize,
    /// Creation time.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Expiry time.
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl Challenge {
    /// Returns true if the challenge has expired.
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() > self.expires_at
    }

    /// Converts this challenge into an AI3 mining task for the given requester.
    pub fn to_mining_task(&self, requester: &str) -> MiningTask {
        MiningTask::new(
            self.operation_type.clone(),
            vec![self.input_tensor.clone()],
            self.difficulty,
            50_000_000, // 50 TRIBE reward
            300,
            requester.to_string(),
        )
    }
}

/// Generates deterministic challenges from (slot, slot_hash) with configurable difficulty and thresholds.
pub struct ChallengeGenerator {
    /// Base difficulty for generated challenges.
    pub base_difficulty: u64,
    /// Base MML threshold.
    pub base_mml_threshold: f64,
    /// Base path distance bound.
    pub base_path_distance: u32,
    /// Maximum tensor dimension.
    pub max_tensor_dim: usize,
    /// Challenge TTL in seconds.
    pub challenge_ttl_secs: i64,
}

impl Default for ChallengeGenerator {
    fn default() -> Self {
        let base_path_distance = NeuralPathValidator::default()
            .layer_widths
            .iter()
            .sum::<usize>() as u32;

        Self {
            base_difficulty: 2,
            base_mml_threshold: 2.0,
            base_path_distance,
            max_tensor_dim: pot_o_core::ESP_MAX_TENSOR_DIM,
            challenge_ttl_secs: 120,
        }
    }
}

/// Available operations weighted by slot hash byte for deterministic selection.
const OPERATIONS: &[&str] = &[
    "matrix_multiply",
    "convolution",
    "relu",
    "sigmoid",
    "tanh",
    "dot_product",
    "normalize",
];

impl ChallengeGenerator {
    /// Creates a generator with the given difficulty and max tensor dimension.
    pub fn new(difficulty: u64, max_tensor_dim: usize) -> Self {
        Self {
            base_difficulty: difficulty,
            max_tensor_dim,
            ..Default::default()
        }
    }

    /// Derive a deterministic challenge from a Solana slot hash.
    pub fn generate(&self, slot: u64, slot_hash_hex: &str) -> TribeResult<Challenge> {
        let hash_bytes = hex::decode(slot_hash_hex).map_err(|e| {
            pot_o_core::TribeError::InvalidOperation(format!("Invalid slot hash hex: {e}"))
        })?;

        let op_index = hash_bytes.first().copied().unwrap_or(0) as usize % OPERATIONS.len();
        let operation_type = OPERATIONS[op_index].to_string();

        let input_tensor = self.derive_input_tensor(&hash_bytes)?;

        let difficulty = self.compute_difficulty(slot);
        let mml_threshold = self.base_mml_threshold / (1.0 + (difficulty as f64).log2().max(0.0));
        let path_distance_max = self
            .base_path_distance
            .saturating_sub((difficulty as u32).min(self.base_path_distance - 1));

        let now = chrono::Utc::now();
        let challenge_id = {
            let mut h = Sha256::new();
            h.update(slot.to_le_bytes());
            h.update(&hash_bytes);
            hex::encode(h.finalize())
        };

        Ok(Challenge {
            id: challenge_id,
            slot,
            slot_hash: slot_hash_hex.to_string(),
            operation_type,
            input_tensor,
            difficulty,
            mml_threshold,
            path_distance_max,
            max_tensor_dim: self.max_tensor_dim,
            created_at: now,
            expires_at: now + chrono::Duration::seconds(self.challenge_ttl_secs),
        })
    }

    /// Build an input tensor from slot hash bytes. Dimensions are clamped to max_tensor_dim.
    fn derive_input_tensor(&self, hash_bytes: &[u8]) -> TribeResult<Tensor> {
        let dim_byte = hash_bytes.get(1).copied().unwrap_or(4);
        let dim = ((dim_byte as usize % self.max_tensor_dim) + 2).min(self.max_tensor_dim);
        let total = dim * dim;

        let mut floats: Vec<f32> = hash_bytes.iter().map(|&b| b as f32 / 255.0).collect();
        // Extend deterministically if hash is shorter than needed
        while floats.len() < total {
            let seed = floats.len() as f32 * 0.618_034;
            floats.push(seed.fract());
        }
        floats.truncate(total);

        Tensor::new(TensorShape::new(vec![dim, dim]), TensorData::F32(floats))
    }

    /// Difficulty scales with slot height (gradual increase).
    fn compute_difficulty(&self, slot: u64) -> u64 {
        let epoch = slot / 10_000;
        self.base_difficulty + epoch.min(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_generation() {
        let gen = ChallengeGenerator::default();
        let hash = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let challenge = gen.generate(100, hash).unwrap();
        assert!(!challenge.id.is_empty());
        assert!(challenge.mml_threshold > 0.0);
        assert!(challenge.mml_threshold <= gen.base_mml_threshold);
    }

    #[test]
    fn test_deterministic_operation() {
        let gen = ChallengeGenerator::default();
        let hash = "ff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
        let c1 = gen.generate(42, hash).unwrap();
        let c2 = gen.generate(42, hash).unwrap();
        assert_eq!(c1.operation_type, c2.operation_type);
        assert_eq!(c1.id, c2.id);
    }
}
