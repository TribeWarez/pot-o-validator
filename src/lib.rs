//! PoT-O Validator library: re-exports of core, ai3-lib, mining, and extensions, plus config.
//!
//! Use this crate when building tooling or tests that need the same types as the validator binary.

pub mod config;

// Re-export with explicit naming to avoid ambiguous glob conflicts
pub use ai3_lib::{
    AI3Engine, ActivationFunction, Convolution, ESPCompatibility, ESPDeviceType, ESPMiningConfig,
    EngineConfig, EngineStats, MatrixMultiply, MinerCapabilities, MinerStats, MiningResult,
    MiningTask, TaskDistributor, Tensor, TensorData, TensorEngine, TensorOp, TensorShape, VectorOp,
};
pub use pot_o_core::{
    approximate_minimal_cut, coherence_probability, effective_distance, entropy_from_cut,
    mutual_information, total_network_entropy, Block, EntanglementEdge, MinimalCut,
    TensorNetworkState, TensorNetworkVertex, Transaction, TransactionType, TribeError, TribeResult,
};
pub use pot_o_extensions::*;
pub use pot_o_mining::*;

/// Crate version (from Cargo.toml).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default HTTP listen port for the validator API.
pub const DEFAULT_PORT: u16 = 8900;
