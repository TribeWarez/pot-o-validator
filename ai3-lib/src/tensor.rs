use pot_o_core::{TribeError, TribeResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorShape {
    pub dims: Vec<usize>,
}

impl TensorShape {
    pub fn new(dims: Vec<usize>) -> Self {
        Self { dims }
    }

    pub fn total_elements(&self) -> usize {
        self.dims.iter().product()
    }

    pub fn is_matrix(&self) -> bool {
        self.dims.len() == 2
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TensorData {
    F32(Vec<f32>),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tensor {
    pub shape: TensorShape,
    pub data: TensorData,
}

impl Tensor {
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

    pub fn zeros(shape: TensorShape) -> Self {
        let n = shape.total_elements();
        Self {
            shape,
            data: TensorData::F32(vec![0.0; n]),
        }
    }

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

    pub fn byte_size(&self) -> usize {
        self.data.to_bytes().len()
    }
}
