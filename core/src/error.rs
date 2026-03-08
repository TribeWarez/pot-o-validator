//! Error types for PoT-O core operations.

use thiserror::Error;

/// Errors that can occur in PoT-O validator and related crates.
#[derive(Error, Debug)]
pub enum TribeError {
    /// Invalid operation or state.
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    /// Proof verification failed.
    #[error("Proof validation failed: {0}")]
    ProofValidationFailed(String),

    /// Tensor or MML operation failed.
    #[error("Tensor operation error: {0}")]
    TensorError(String),

    /// Cross-chain or bridge operation failed.
    #[error("Chain bridge error: {0}")]
    ChainBridgeError(String),

    /// Network or RPC error.
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Invalid or missing configuration.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Device or protocol error.
    #[error("Device protocol error: {0}")]
    DeviceError(String),

    /// Serialization or deserialization failed.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// I/O error (e.g. file or network).
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Tensor network operation failed or reached capacity
    #[error("Tensor network error: {0}")]
    TensorNetworkError(String),

    /// Tensor network is at maximum capacity
    #[error("Tensor network is full: cannot add more vertices/edges")]
    TensorNetworkFull,
}

/// Result type alias using [`TribeError`].
pub type TribeResult<T> = Result<T, TribeError>;
