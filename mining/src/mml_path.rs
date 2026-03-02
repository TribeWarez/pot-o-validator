use ai3_lib::tensor::Tensor;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use pot_o_core::TribeResult;
use std::io::Write;

/// Validates a tensor transformation using Minimum Message Length (Kolmogorov) principles.
///
/// The MML score measures how well the output compresses relative to the input.
/// A lower score means the transformation found a more optimal encoding path.
/// The proof is valid only if `mml_score <= mml_threshold`.
pub struct MMLPathValidator {
    pub compression_level: u32,
}

impl Default for MMLPathValidator {
    fn default() -> Self {
        Self {
            compression_level: 6,
        }
    }
}

impl MMLPathValidator {
    /// Compute the MML score: ratio of compressed output length to compressed input length.
    /// Score < 1.0 means the transformation produced a more compressible result.
    pub fn compute_mml_score(&self, input: &Tensor, output: &Tensor) -> TribeResult<f64> {
        let input_compressed_len = self.compressed_length(&input.data.to_bytes())?;
        let output_compressed_len = self.compressed_length(&output.data.to_bytes())?;

        if input_compressed_len == 0 {
            return Ok(1.0);
        }

        Ok(output_compressed_len as f64 / input_compressed_len as f64)
    }

    /// Compute an MML-like score using the same byte-level entropy approximation
    /// implemented on ESP firmware. This does NOT use DEFLATE and is intended
    /// for calibration and comparison only.
    ///
    /// Score is defined as: output_entropy / input_entropy, where entropy is
    /// Shannon entropy over the raw little-endian bytes of the tensor data.
    pub fn compute_entropy_mml_score(&self, input: &Tensor, output: &Tensor) -> f64 {
        fn entropy(bytes: &[u8]) -> f64 {
            let mut hist = [0u64; 256];
            for &b in bytes {
                hist[b as usize] += 1;
            }
            let total = bytes.len() as f64;
            if total == 0.0 {
                return 0.0;
            }
            let mut ent = 0.0f64;
            for &count in &hist {
                if count == 0 {
                    continue;
                }
                let p = count as f64 / total;
                ent -= p * p.ln();
            }
            ent
        }

        let input_bytes = input.data.to_bytes();
        let output_bytes = output.data.to_bytes();
        let in_ent = entropy(&input_bytes);
        let out_ent = entropy(&output_bytes);
        if in_ent.abs() < f64::EPSILON {
            1.0
        } else {
            out_ent / in_ent
        }
    }

    /// Check whether an MML score passes the threshold for a given difficulty.
    pub fn validate(&self, mml_score: f64, mml_threshold: f64) -> bool {
        mml_score <= mml_threshold
    }

    /// Compute the MML threshold for a given difficulty level.
    /// Tighter (lower) thresholds at higher difficulties.
    pub fn threshold_for_difficulty(base_threshold: f64, difficulty: u64) -> f64 {
        base_threshold / (1.0 + (difficulty as f64).log2().max(0.0))
    }

    fn compressed_length(&self, data: &[u8]) -> TribeResult<usize> {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(self.compression_level));
        encoder
            .write_all(data)
            .map_err(|e| pot_o_core::TribeError::TensorError(format!("Compression failed: {e}")))?;
        let compressed = encoder
            .finish()
            .map_err(|e| pot_o_core::TribeError::TensorError(format!("Compression failed: {e}")))?;
        Ok(compressed.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai3_lib::tensor::{TensorData, TensorShape};

    #[test]
    fn test_mml_score_computation() {
        let validator = MMLPathValidator::default();

        let input = Tensor::new(
            TensorShape::new(vec![4]),
            TensorData::F32(vec![1.0, 2.0, 3.0, 4.0]),
        )
        .unwrap();

        // Zeros compress better than random data
        let output = Tensor::new(
            TensorShape::new(vec![4]),
            TensorData::F32(vec![0.0, 0.0, 0.0, 0.0]),
        )
        .unwrap();

        let score = validator.compute_mml_score(&input, &output).unwrap();
        assert!(score > 0.0, "Score should be positive");
    }

    #[test]
    fn test_threshold_scaling() {
        let t1 = MMLPathValidator::threshold_for_difficulty(0.85, 1);
        let t4 = MMLPathValidator::threshold_for_difficulty(0.85, 4);
        let t8 = MMLPathValidator::threshold_for_difficulty(0.85, 8);
        assert!(t4 < t1, "Higher difficulty should give lower threshold");
        assert!(t8 < t4);
    }
}
