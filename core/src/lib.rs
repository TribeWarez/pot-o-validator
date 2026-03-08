//! Core types and utilities for PoT-O (Proof of Tensor Optimizations).
//!
//! Provides block and transaction types, error handling, tensor network utilities,
//! and constants used across the validator, mining, and extensions crates.
//!
//! # Tensor Network (REALMS Part IV)
//!
//! This crate implements quantum-inspired tensor network models where:
//! - Vertices represent quantum subsystems (miners, pools)
//! - Edges represent entanglement links with bond dimension d
//! - Entropy S(A) = |γ_A| * log(d) quantifies entanglement
//! - Mutual information I(A:B) measures region coupling
//! - Effective distance d_eff recovers geometric structure

pub mod error;
pub mod math;
pub mod tensor;
pub mod types;

pub use error::{TribeError, TribeResult};
pub use math::portable::*;
pub use tensor::{
    constants::*,
    entropy::{
        approximate_minimal_cut, coherence_probability, effective_distance, entropy_from_cut,
        mutual_information, total_network_entropy,
    },
};
pub use types::*;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// TribeChain version (from crate version).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Block time target in seconds.
pub const BLOCK_TIME_TARGET: u64 = 60;

/// Maximum tensor dimensions for ESP-compatible challenges.
pub const ESP_MAX_TENSOR_DIM: usize = 64;

/// Token type identifier on the chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TokenType {
    /// Native chain token.
    TribeChain,
    /// Pumped TRIB€-test Coin (mining rewards).
    PTtC,
    /// Numerologic Master Coin.
    NMTC,
    /// STOMP token.
    STOMP,
    /// AUM token.
    AUM,
    /// AI3 token.
    AI3,
}

/// Minimal block representation aligned with .AI3 core::Block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Block height (genesis = 0).
    pub height: u64,
    /// SHA-256 hash of the block header and transactions.
    pub hash: String,
    /// Hash of the previous block.
    pub previous_hash: String,
    /// Unix timestamp.
    pub timestamp: u64,
    /// Proof nonce.
    pub nonce: u64,
    /// Mining difficulty target.
    pub difficulty: u32,
    /// Miner address or identifier.
    pub miner: String,
    /// Transactions included in this block.
    pub transactions: Vec<Transaction>,
}

impl Block {
    /// Builds a new block with computed hash.
    pub fn new(
        height: u64,
        previous_hash: String,
        transactions: Vec<Transaction>,
        miner: String,
        difficulty: u32,
    ) -> Self {
        let mut block = Self {
            height,
            hash: String::new(),
            previous_hash,
            timestamp: chrono::Utc::now().timestamp() as u64,
            nonce: 0,
            difficulty,
            miner,
            transactions,
        };
        block.hash = block.calculate_hash();
        block
    }

    /// Computes the block hash from header and transaction hashes.
    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.height.to_le_bytes());
        hasher.update(self.previous_hash.as_bytes());
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(self.nonce.to_le_bytes());
        hasher.update(self.difficulty.to_le_bytes());
        hasher.update(self.miner.as_bytes());
        for tx in &self.transactions {
            hasher.update(tx.hash.as_bytes());
        }
        hex::encode(hasher.finalize())
    }
}

/// A single chain transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction hash.
    pub hash: String,
    /// Sender address.
    pub from: String,
    /// Recipient address.
    pub to: String,
    /// Amount (in smallest unit).
    pub amount: u64,
    /// Fee paid.
    pub fee: u64,
    /// Unix timestamp.
    pub timestamp: u64,
    /// Sender nonce.
    pub nonce: u64,
    /// Transaction kind.
    pub tx_type: TransactionType,
}

/// Kind of on-chain transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    /// Simple transfer.
    Transfer,
    /// Staking operation.
    Stake,
    /// PoT-O tensor proof submission.
    TensorProof,
    /// Token creation.
    TokenCreate,
    /// AMM swap.
    Swap,
}
