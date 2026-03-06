//! Pool strategy: solo and proportional/PPLNS pool info and reward distribution.

use pot_o_core::TribeResult;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolType {
    Solo,
    Proportional,
    PPLNS,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerShare {
    pub miner_pubkey: String,
    pub share_pct: f64,
    pub reward_amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerInfo {
    pub pubkey: String,
    pub stake: u64,
    pub proofs_submitted: u64,
    pub reputation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRecord {
    pub miner_pubkey: String,
    pub challenge_id: String,
    pub reward: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PoolInfo {
    pub pool_type: String,
    pub total_miners: usize,
    pub total_stake: u64,
    pub minimum_stake: u64,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// How mining rewards are distributed among participants.
pub trait PoolStrategy: Send + Sync {
    fn pool_type(&self) -> PoolType;
    fn calculate_shares(&self, proofs: &[ProofRecord], reward: u64)
        -> TribeResult<Vec<MinerShare>>;
    fn minimum_stake(&self) -> u64;
    fn accept_miner(&self, miner: &MinerInfo) -> TribeResult<bool>;
    fn pool_info(&self, miners: usize, total_stake: u64) -> PoolInfo;
}

// ---------------------------------------------------------------------------
// SoloStrategy (implemented now)
// ---------------------------------------------------------------------------

pub struct SoloStrategy;

impl PoolStrategy for SoloStrategy {
    fn pool_type(&self) -> PoolType {
        PoolType::Solo
    }

    fn calculate_shares(
        &self,
        proofs: &[ProofRecord],
        reward: u64,
    ) -> TribeResult<Vec<MinerShare>> {
        // 100% to the miner who submitted the proof
        Ok(proofs
            .last()
            .map(|p| {
                vec![MinerShare {
                    miner_pubkey: p.miner_pubkey.clone(),
                    share_pct: 100.0,
                    reward_amount: reward,
                }]
            })
            .unwrap_or_default())
    }

    fn minimum_stake(&self) -> u64 {
        0 // No stake required for solo
    }

    fn accept_miner(&self, _miner: &MinerInfo) -> TribeResult<bool> {
        Ok(true)
    }

    fn pool_info(&self, miners: usize, total_stake: u64) -> PoolInfo {
        PoolInfo {
            pool_type: "solo".into(),
            total_miners: miners,
            total_stake,
            minimum_stake: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// ProportionalPool (stubbed)
// ---------------------------------------------------------------------------

pub struct ProportionalPool {
    pub min_stake: u64,
}

impl PoolStrategy for ProportionalPool {
    fn pool_type(&self) -> PoolType {
        PoolType::Proportional
    }

    fn calculate_shares(
        &self,
        _proofs: &[ProofRecord],
        _reward: u64,
    ) -> TribeResult<Vec<MinerShare>> {
        todo!("Proportional reward distribution not yet implemented")
    }

    fn minimum_stake(&self) -> u64 {
        self.min_stake
    }

    fn accept_miner(&self, miner: &MinerInfo) -> TribeResult<bool> {
        Ok(miner.stake >= self.min_stake)
    }

    fn pool_info(&self, miners: usize, total_stake: u64) -> PoolInfo {
        PoolInfo {
            pool_type: "proportional".into(),
            total_miners: miners,
            total_stake,
            minimum_stake: self.min_stake,
        }
    }
}

// ---------------------------------------------------------------------------
// PPLNSPool (stubbed)
// ---------------------------------------------------------------------------

pub struct PPLNSPool {
    pub window_size: usize,
    pub min_stake: u64,
}

impl PoolStrategy for PPLNSPool {
    fn pool_type(&self) -> PoolType {
        PoolType::PPLNS
    }

    fn calculate_shares(
        &self,
        _proofs: &[ProofRecord],
        _reward: u64,
    ) -> TribeResult<Vec<MinerShare>> {
        todo!("PPLNS reward distribution not yet implemented")
    }

    fn minimum_stake(&self) -> u64 {
        self.min_stake
    }

    fn accept_miner(&self, miner: &MinerInfo) -> TribeResult<bool> {
        Ok(miner.stake >= self.min_stake)
    }

    fn pool_info(&self, miners: usize, total_stake: u64) -> PoolInfo {
        PoolInfo {
            pool_type: "pplns".into(),
            total_miners: miners,
            total_stake,
            minimum_stake: self.min_stake,
        }
    }
}
