use thiserror::Error;

#[derive(Error, Debug)]
pub enum TribeError {
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Proof validation failed: {0}")]
    ProofValidationFailed(String),

    #[error("Tensor operation error: {0}")]
    TensorError(String),

    #[error("Chain bridge error: {0}")]
    ChainBridgeError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Device protocol error: {0}")]
    DeviceError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type TribeResult<T> = Result<T, TribeError>;
