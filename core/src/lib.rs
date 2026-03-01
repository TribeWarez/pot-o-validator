pub mod error;

pub use error::{TribeError, TribeResult};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// TribeChain version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Block time target (seconds)
pub const BLOCK_TIME_TARGET: u64 = 60;

/// Max tensor dimensions for ESP-compatible challenges
pub const ESP_MAX_TENSOR_DIM: usize = 64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TokenType {
    TribeChain,
    PTtC,
    NMTC,
    STOMP,
    AUM,
    AI3,
}

/// Minimal block representation aligned with .AI3 core::Block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub height: u64,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: u64,
    pub nonce: u64,
    pub difficulty: u32,
    pub miner: String,
    pub transactions: Vec<Transaction>,
}

impl Block {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: u64,
    pub nonce: u64,
    pub tx_type: TransactionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Transfer,
    Stake,
    TensorProof,
    TokenCreate,
    Swap,
}
