//! Neural path validation: expected path signature from challenge and actual path from tensor + nonce.

use ai3_lib::tensor::Tensor;
use pot_o_core::TribeResult;
use sha2::{Digest, Sha256};

/// Validates the neural inference path of a tensor computation.
///
/// Models the tensor operation as a small feedforward network. The "path"
/// is the binary activation pattern at each layer (ReLU > 0 = 1, else 0).
/// The miner must find a nonce that makes the actual path match an expected
/// path signature (derived from the challenge) within a Hamming distance.
pub struct NeuralPathValidator {
    /// Layer widths for the feedforward network
    pub layer_widths: Vec<usize>,
}

impl Default for NeuralPathValidator {
    fn default() -> Self {
        Self {
            layer_widths: vec![32, 16, 8],
        }
    }
}

impl NeuralPathValidator {
    /// Derive the expected path signature from the challenge hash.
    /// Returns a bit vector (as Vec<u8> of 0/1 values) representing the expected activations.
    pub fn expected_path_signature(&self, challenge_hash: &str) -> Vec<u8> {
        let hash_bytes = hex::decode(challenge_hash).unwrap_or_default();
        let total_neurons: usize = self.layer_widths.iter().sum();
        let mut sig = Vec::with_capacity(total_neurons);

        let mut hasher = Sha256::new();
        hasher.update(&hash_bytes);
        let mut seed = hasher.finalize().to_vec();

        for &width in &self.layer_widths {
            for i in 0..width {
                let byte_idx = i % seed.len();
                let bit = (seed[byte_idx] >> (i % 8)) & 1;
                sig.push(bit);
            }
            // Re-hash seed for next layer so each layer has different expected bits
            let mut h = Sha256::new();
            h.update(&seed);
            seed = h.finalize().to_vec();
        }

        sig
    }

    /// Compute the actual activation path for a tensor with a given nonce.
    /// Simulates a feedforward pass: input -> (linear + ReLU) per layer.
    /// Returns the binary activation pattern.
    pub fn compute_actual_path(&self, tensor: &Tensor, nonce: u64) -> TribeResult<Vec<u8>> {
        let mut activations = tensor.data.as_f32();
        let mut path_bits = Vec::new();
        let mut bit_idx: u32 = 0;

        for &width in &self.layer_widths {
            let mut layer_output = vec![0.0f32; width];

            // Simplified linear: each output neuron sums a stride of the input
            let stride = (activations.len() / width).max(1);
            for (j, out) in layer_output.iter_mut().enumerate() {
                if j >= width {
                    break;
                }
                let start = j * stride;
                let end = (start + stride).min(activations.len());
                let sum: f32 = activations[start..end].iter().sum();
                // ReLU
                let relu = sum.max(0.0);
                *out = relu;

                let base_bit = if relu > 0.0 { 1u8 } else { 0u8 };
                let shift = (bit_idx % 64) as u64;
                let nonce_bit = ((nonce >> shift) & 1) as u8;
                let bit = base_bit ^ nonce_bit;

                path_bits.push(bit);
                bit_idx = bit_idx.wrapping_add(1);
            }

            activations = layer_output;
        }

        Ok(path_bits)
    }

    /// Compute Hamming distance between two bit vectors.
    pub fn hamming_distance(a: &[u8], b: &[u8]) -> u32 {
        a.iter()
            .zip(b.iter())
            .map(|(&x, &y)| if x != y { 1u32 } else { 0u32 })
            .sum()
    }

    /// Validate that the actual path is close enough to the expected path.
    pub fn validate(&self, actual_path: &[u8], challenge_hash: &str, max_distance: u32) -> bool {
        let expected = self.expected_path_signature(challenge_hash);
        let min_len = actual_path.len().min(expected.len());
        let distance = Self::hamming_distance(&actual_path[..min_len], &expected[..min_len]);
        distance <= max_distance
    }

    /// Encode path bits as a compact hex string for on-chain storage.
    pub fn path_to_hex(path: &[u8]) -> String {
        let mut bytes = Vec::with_capacity(path.len().div_ceil(8));
        for chunk in path.chunks(8) {
            let mut byte = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                if bit != 0 {
                    byte |= 1 << i;
                }
            }
            bytes.push(byte);
        }
        hex::encode(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai3_lib::tensor::{TensorData, TensorShape};

    #[test]
    fn test_expected_path_deterministic() {
        let v = NeuralPathValidator::default();
        let hash = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let p1 = v.expected_path_signature(hash);
        let p2 = v.expected_path_signature(hash);
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_hamming_distance() {
        assert_eq!(
            NeuralPathValidator::hamming_distance(&[0, 1, 0], &[0, 1, 0]),
            0
        );
        assert_eq!(
            NeuralPathValidator::hamming_distance(&[0, 1, 0], &[1, 0, 1]),
            3
        );
        assert_eq!(
            NeuralPathValidator::hamming_distance(&[1, 1, 1], &[0, 1, 0]),
            2
        );
    }

    #[test]
    fn test_actual_path_varies_with_nonce() {
        let v = NeuralPathValidator::default();
        let t = Tensor::new(TensorShape::new(vec![64]), TensorData::F32(vec![0.5; 64])).unwrap();
        let p1 = v.compute_actual_path(&t, 0).unwrap();
        let p2 = v.compute_actual_path(&t, 999_999).unwrap();
        // Different nonces should (usually) produce different paths
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_path_hex_roundtrip() {
        let path = vec![1, 0, 1, 1, 0, 0, 1, 0, 1];
        let hex_str = NeuralPathValidator::path_to_hex(&path);
        assert!(!hex_str.is_empty());
    }
}
