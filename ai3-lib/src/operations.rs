use crate::tensor::{Tensor, TensorData, TensorShape};
use pot_o_core::{TribeError, TribeResult};

/// Trait for all tensor operations (aligned with .AI3 TensorOp)
pub trait TensorOp: Send + Sync {
    fn name(&self) -> &str;
    fn execute(&self, input: &Tensor) -> TribeResult<Tensor>;
}

pub fn parse_operation(op_type: &str) -> TribeResult<Box<dyn TensorOp>> {
    match op_type {
        "matrix_multiply" => Ok(Box::new(MatrixMultiply)),
        "convolution" => Ok(Box::new(Convolution::default())),
        "relu" => Ok(Box::new(ActivationFunction::ReLU)),
        "sigmoid" => Ok(Box::new(ActivationFunction::Sigmoid)),
        "tanh" => Ok(Box::new(ActivationFunction::Tanh)),
        "dot_product" => Ok(Box::new(VectorOp::DotProduct)),
        "normalize" => Ok(Box::new(VectorOp::Normalize)),
        _ => Err(TribeError::TensorError(format!(
            "Unknown operation: {op_type}"
        ))),
    }
}

/// Matrix multiplication (self-multiply for square-ish inputs)
pub struct MatrixMultiply;

impl TensorOp for MatrixMultiply {
    fn name(&self) -> &str {
        "matrix_multiply"
    }

    fn execute(&self, input: &Tensor) -> TribeResult<Tensor> {
        let data = input.data.as_f32();
        let n = (data.len() as f64).sqrt() as usize;
        if n == 0 {
            return Ok(Tensor::zeros(TensorShape::new(vec![0])));
        }
        let size = n * n;
        let a: Vec<f32> = data.iter().copied().take(size).collect();
        let mut result = vec![0.0f32; size];
        for i in 0..n {
            for j in 0..n {
                let mut sum = 0.0f32;
                for k in 0..n {
                    let ai = a.get(i * n + k).copied().unwrap_or(0.0);
                    let bj = a.get(k * n + j).copied().unwrap_or(0.0);
                    sum += ai * bj;
                }
                result[i * n + j] = sum;
            }
        }
        Tensor::new(TensorShape::new(vec![n, n]), TensorData::F32(result))
    }
}

/// 1D convolution with a small fixed kernel
pub struct Convolution {
    pub kernel: Vec<f32>,
}

impl Default for Convolution {
    fn default() -> Self {
        Self {
            kernel: vec![0.25, 0.5, 0.25],
        }
    }
}

impl TensorOp for Convolution {
    fn name(&self) -> &str {
        "convolution"
    }

    fn execute(&self, input: &Tensor) -> TribeResult<Tensor> {
        let data = input.data.as_f32();
        let klen = self.kernel.len();
        if data.len() < klen {
            return Ok(input.clone());
        }
        let out_len = data.len() - klen + 1;
        let mut result = Vec::with_capacity(out_len);
        for i in 0..out_len {
            let mut sum = 0.0f32;
            for (j, &kv) in self.kernel.iter().enumerate() {
                sum += data[i + j] * kv;
            }
            result.push(sum);
        }
        Tensor::new(TensorShape::new(vec![out_len]), TensorData::F32(result))
    }
}

#[derive(Debug, Clone)]
pub enum ActivationFunction {
    ReLU,
    Sigmoid,
    Tanh,
}

impl TensorOp for ActivationFunction {
    fn name(&self) -> &str {
        match self {
            Self::ReLU => "relu",
            Self::Sigmoid => "sigmoid",
            Self::Tanh => "tanh",
        }
    }

    fn execute(&self, input: &Tensor) -> TribeResult<Tensor> {
        let data = input.data.as_f32();
        let result: Vec<f32> = match self {
            Self::ReLU => data.iter().map(|&x| x.max(0.0)).collect(),
            Self::Sigmoid => data.iter().map(|&x| 1.0 / (1.0 + (-x).exp())).collect(),
            Self::Tanh => data.iter().map(|&x| x.tanh()).collect(),
        };
        Tensor::new(input.shape.clone(), TensorData::F32(result))
    }
}

#[derive(Debug, Clone)]
pub enum VectorOp {
    DotProduct,
    Normalize,
}

impl TensorOp for VectorOp {
    fn name(&self) -> &str {
        match self {
            Self::DotProduct => "dot_product",
            Self::Normalize => "normalize",
        }
    }

    fn execute(&self, input: &Tensor) -> TribeResult<Tensor> {
        let data = input.data.as_f32();
        match self {
            Self::DotProduct => {
                let half = data.len() / 2;
                let dot: f32 = data[..half]
                    .iter()
                    .zip(data[half..half * 2].iter())
                    .map(|(a, b)| a * b)
                    .sum();
                Tensor::new(TensorShape::new(vec![1]), TensorData::F32(vec![dot]))
            }
            Self::Normalize => {
                let magnitude: f32 = data.iter().map(|x| x * x).sum::<f32>().sqrt();
                let result = if magnitude > f32::EPSILON {
                    data.iter().map(|x| x / magnitude).collect()
                } else {
                    data.clone()
                };
                Tensor::new(input.shape.clone(), TensorData::F32(result))
            }
        }
    }
}
