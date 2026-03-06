//! Tensor types: shape, data (F32/U8), and operations (hash, clamp for ESP).

use pot_o_core::{TribeError, TribeResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Shape of a tensor (dimension sizes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorShape {
    /// Dimension sizes (e.g. [8, 8] for 8x8 matrix).
    pub dims: Vec<usize>,
}

impl TensorShape {
    /// Creates a shape from dimension sizes.
    pub fn new(dims: Vec<usize>) -> Self {
        Self { dims }
    }

    /// Total number of elements (product of dims).
    pub fn total_elements(&self) -> usize {
        self.dims.iter().product()
    }

    /// True if 2D (matrix).
    pub fn is_matrix(&self) -> bool {
        self.dims.len() == 2
    }
}

/// Tensor element data (float or u8).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TensorData {
    /// 32-bit float elements.
    F32(Vec<f32>),
    /// 8-bit unsigned elements.
    U8(Vec<u8>),
}

impl TensorData {
    pub fn len(&self) -> usize {
        match self {
            Self::F32(v) => v.len(),
            Self::U8(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_f32(&self) -> Vec<f32> {
        match self {
            Self::F32(v) => v.clone(),
            Self::U8(v) => v.iter().map(|&b| b as f32 / 255.0).collect(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::F32(v) => v.iter().flat_map(|f| f.to_le_bytes()).collect(),
            Self::U8(v) => v.clone(),
        }
    }
}

/// A typed tensor with shape and data (used in challenges and mining results).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tensor {
    /// Shape (dimension sizes).
    pub shape: TensorShape,
    /// Element data.
    pub data: TensorData,
}

impl Tensor {
    /// Builds a tensor; errors if data length does not match shape.
    pub fn new(shape: TensorShape, data: TensorData) -> TribeResult<Self> {
        let expected = shape.total_elements();
        let actual = data.len();
        if actual != expected {
            return Err(TribeError::TensorError(format!(
                "Shape expects {expected} elements but data has {actual}"
            )));
        }
        Ok(Self { shape, data })
    }

    /// Creates a tensor of zeros with the given shape.
    pub fn zeros(shape: TensorShape) -> Self {
        let n = shape.total_elements();
        Self {
            shape,
            data: TensorData::F32(vec![0.0; n]),
        }
    }

    /// Builds a 1D tensor from raw hash bytes (normalized to 0..1).
    pub fn from_slot_hash(hash_bytes: &[u8]) -> Self {
        let floats: Vec<f32> = hash_bytes.iter().map(|&b| b as f32 / 255.0).collect();
        let n = floats.len();
        Self {
            shape: TensorShape::new(vec![n]),
            data: TensorData::F32(floats),
        }
    }

    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.data.to_bytes());
        for d in &self.shape.dims {
            hasher.update(d.to_le_bytes());
        }
        hex::encode(hasher.finalize())
    }

    /// Clamp tensor dimensions to a maximum size (for ESP compatibility).
    pub fn clamp_dimensions(&self, max_dim: usize) -> Self {
        let floats = self.data.as_f32();
        let clamped_len = floats.len().min(max_dim * max_dim);
        let clamped_data: Vec<f32> = floats.into_iter().take(clamped_len).collect();
        let new_shape = self.shape.dims.iter().map(|&d| d.min(max_dim)).collect();
        Self {
            shape: TensorShape::new(new_shape),
            data: TensorData::F32(clamped_data),
        }
    }

    /// Serialized byte length of the tensor data.
    pub fn byte_size(&self) -> usize {
        self.data.to_bytes().len()
    }
}
