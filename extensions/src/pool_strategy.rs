//! Pool strategy: solo and proportional/PPLNS pool info and reward distribution.

use pot_o_core::TribeResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
// Helper function for fair reward distribution with proper rounding
// ---------------------------------------------------------------------------

fn distribute_rewards(
    miner_counts: HashMap<String, u64>,
    total_proofs: u64,
    total_reward: u64,
) -> Vec<MinerShare> {
    let mut shares = Vec::new();

    // First pass: calculate rewards (truncating)
    let mut calculated_rewards: Vec<(String, u64, f64)> = miner_counts
        .into_iter()
        .map(|(miner_pubkey, count)| {
            let share_pct = (count as f64 / total_proofs as f64) * 100.0;
            let reward_amount = (count as f64 / total_proofs as f64 * total_reward as f64) as u64;
            (miner_pubkey, reward_amount, share_pct)
        })
        .collect();

    // Calculate total distributed and remainder
    let total_distributed: u64 = calculated_rewards.iter().map(|r| r.1).sum();
    let remainder = total_reward - total_distributed;

    // Give remainder to the last miner (ensures total == reward)
    if let Some(last) = calculated_rewards.last_mut() {
        last.1 += remainder;
    }

    // Convert to MinerShare
    for (miner_pubkey, reward_amount, share_pct) in calculated_rewards {
        shares.push(MinerShare {
            miner_pubkey,
            share_pct,
            reward_amount,
        });
    }

    shares
}

// ---------------------------------------------------------------------------
// ProportionalPool (implemented)
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
        proofs: &[ProofRecord],
        reward: u64,
    ) -> TribeResult<Vec<MinerShare>> {
        if proofs.is_empty() {
            return Ok(Vec::new());
        }

        // Count proofs per miner
        let mut miner_counts: HashMap<String, u64> = HashMap::new();
        for proof in proofs {
            *miner_counts.entry(proof.miner_pubkey.clone()).or_insert(0) += 1;
        }

        let total_proofs = proofs.len() as u64;
        let shares = distribute_rewards(miner_counts, total_proofs, reward);

        Ok(shares)
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
        proofs: &[ProofRecord],
        reward: u64,
    ) -> TribeResult<Vec<MinerShare>> {
        if proofs.is_empty() {
            return Ok(Vec::new());
        }

        // Take only the last window_size proofs
        let start_idx = if proofs.len() > self.window_size {
            proofs.len() - self.window_size
        } else {
            0
        };
        let window_proofs = &proofs[start_idx..];

        // Count proofs per miner in the window
        let mut miner_counts: HashMap<String, u64> = HashMap::new();
        for proof in window_proofs {
            *miner_counts.entry(proof.miner_pubkey.clone()).or_insert(0) += 1;
        }

        let total_proofs = window_proofs.len() as u64;
        let shares = distribute_rewards(miner_counts, total_proofs, reward);

        Ok(shares)
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_proof(miner_pubkey: &str) -> ProofRecord {
        ProofRecord {
            miner_pubkey: miner_pubkey.to_string(),
            challenge_id: "challenge1".to_string(),
            reward: 0,
            timestamp: Utc::now(),
        }
    }

    fn create_miner(pubkey: &str, stake: u64) -> MinerInfo {
        MinerInfo {
            pubkey: pubkey.to_string(),
            stake,
            proofs_submitted: 0,
            reputation: 1.0,
        }
    }

    // =========================================================================
    // ProportionalPool Tests
    // =========================================================================

    #[test]
    fn test_proportional_single_miner() {
        let strategy = ProportionalPool { min_stake: 0 };
        let proofs = vec![
            create_proof("miner_a"),
            create_proof("miner_a"),
            create_proof("miner_a"),
        ];

        let shares = strategy.calculate_shares(&proofs, 1000).unwrap();

        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].miner_pubkey, "miner_a");
        assert_eq!(shares[0].share_pct, 100.0);
        assert_eq!(shares[0].reward_amount, 1000);
    }

    #[test]
    fn test_proportional_equal_shares() {
        let strategy = ProportionalPool { min_stake: 0 };
        let proofs = vec![
            create_proof("miner_a"),
            create_proof("miner_b"),
            create_proof("miner_a"),
            create_proof("miner_b"),
        ];

        let shares = strategy.calculate_shares(&proofs, 1000).unwrap();

        assert_eq!(shares.len(), 2);

        // Find miner_a
        let miner_a = shares.iter().find(|s| s.miner_pubkey == "miner_a").unwrap();
        assert_eq!(miner_a.share_pct, 50.0);
        assert_eq!(miner_a.reward_amount, 500);

        // Find miner_b
        let miner_b = shares.iter().find(|s| s.miner_pubkey == "miner_b").unwrap();
        assert_eq!(miner_b.share_pct, 50.0);
        assert_eq!(miner_b.reward_amount, 500);
    }

    #[test]
    fn test_proportional_unequal_shares() {
        let strategy = ProportionalPool { min_stake: 0 };
        let proofs = vec![
            create_proof("miner_a"),
            create_proof("miner_b"),
            create_proof("miner_a"),
            create_proof("miner_c"),
            create_proof("miner_a"),
        ];

        let shares = strategy.calculate_shares(&proofs, 1000).unwrap();

        assert_eq!(shares.len(), 3);

        // miner_a: 3/5 = 60%
        let miner_a = shares.iter().find(|s| s.miner_pubkey == "miner_a").unwrap();
        assert_eq!(miner_a.share_pct, 60.0);
        assert_eq!(miner_a.reward_amount, 600);

        // miner_b: 1/5 = 20%
        let miner_b = shares.iter().find(|s| s.miner_pubkey == "miner_b").unwrap();
        assert_eq!(miner_b.share_pct, 20.0);
        assert_eq!(miner_b.reward_amount, 200);

        // miner_c: 1/5 = 20%
        let miner_c = shares.iter().find(|s| s.miner_pubkey == "miner_c").unwrap();
        assert_eq!(miner_c.share_pct, 20.0);
        assert_eq!(miner_c.reward_amount, 200);
    }

    #[test]
    fn test_proportional_empty_proofs() {
        let strategy = ProportionalPool { min_stake: 0 };
        let proofs = vec![];

        let shares = strategy.calculate_shares(&proofs, 1000).unwrap();

        assert_eq!(shares.len(), 0);
    }

    #[test]
    fn test_proportional_fractional_rounding() {
        let strategy = ProportionalPool { min_stake: 0 };
        let proofs = vec![
            create_proof("miner_a"),
            create_proof("miner_b"),
            create_proof("miner_c"),
        ];

        let shares = strategy.calculate_shares(&proofs, 1000).unwrap();

        assert_eq!(shares.len(), 3);

        // Each miner should get approximately 333.33
        let total: u64 = shares.iter().map(|s| s.reward_amount).sum();
        assert_eq!(total, 1000); // Total must equal original reward

        // Percentages should sum to 100.0
        let pct_sum: f64 = shares.iter().map(|s| s.share_pct).sum();
        assert!((pct_sum - 100.0).abs() < 0.01);
    }

    // =========================================================================
    // PPLNSPool Tests
    // =========================================================================

    #[test]
    fn test_pplns_window_excludes_old() {
        let strategy = PPLNSPool {
            window_size: 3,
            min_stake: 0,
        };
        let proofs = vec![
            create_proof("miner_old1"),
            create_proof("miner_old2"),
            create_proof("miner_a"),
            create_proof("miner_b"),
            create_proof("miner_a"),
        ];

        let shares = strategy.calculate_shares(&proofs, 1000).unwrap();

        // Should only include last 3: [miner_a, miner_b, miner_a]
        // miner_old1 and miner_old2 should not be in results
        let has_old1 = shares.iter().any(|s| s.miner_pubkey == "miner_old1");
        let has_old2 = shares.iter().any(|s| s.miner_pubkey == "miner_old2");

        assert!(!has_old1);
        assert!(!has_old2);

        // Check total rewards equal the input reward
        let total: u64 = shares.iter().map(|s| s.reward_amount).sum();
        assert_eq!(total, 1000);

        // miner_a: 2/3 = ~666.67%
        let miner_a = shares.iter().find(|s| s.miner_pubkey == "miner_a").unwrap();
        assert!((miner_a.share_pct - 66.67).abs() < 0.1);
        assert!(miner_a.reward_amount >= 666 && miner_a.reward_amount <= 667);

        // miner_b: 1/3 = ~333.33%
        let miner_b = shares.iter().find(|s| s.miner_pubkey == "miner_b").unwrap();
        assert!((miner_b.share_pct - 33.33).abs() < 0.1);
        assert!(miner_b.reward_amount >= 333 && miner_b.reward_amount <= 334);
    }

    #[test]
    fn test_pplns_single_miner_in_window() {
        let strategy = PPLNSPool {
            window_size: 5,
            min_stake: 0,
        };
        let proofs = vec![
            create_proof("miner_a"),
            create_proof("miner_a"),
            create_proof("miner_a"),
        ];

        let shares = strategy.calculate_shares(&proofs, 1000).unwrap();

        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].miner_pubkey, "miner_a");
        assert_eq!(shares[0].share_pct, 100.0);
        assert_eq!(shares[0].reward_amount, 1000);
    }

    #[test]
    fn test_pplns_shared_window() {
        let strategy = PPLNSPool {
            window_size: 4,
            min_stake: 0,
        };
        let proofs = vec![
            create_proof("miner_a"),
            create_proof("miner_a"),
            create_proof("miner_b"),
            create_proof("miner_c"),
            create_proof("miner_b"),
        ];

        let shares = strategy.calculate_shares(&proofs, 1000).unwrap();

        // Last 4 proofs: [miner_a, miner_b, miner_c, miner_b]
        // miner_a: 1/4 = 25%
        // miner_b: 2/4 = 50%
        // miner_c: 1/4 = 25%

        assert_eq!(shares.len(), 3);

        let miner_a = shares.iter().find(|s| s.miner_pubkey == "miner_a").unwrap();
        assert_eq!(miner_a.share_pct, 25.0);
        assert_eq!(miner_a.reward_amount, 250);

        let miner_b = shares.iter().find(|s| s.miner_pubkey == "miner_b").unwrap();
        assert_eq!(miner_b.share_pct, 50.0);
        assert_eq!(miner_b.reward_amount, 500);

        let miner_c = shares.iter().find(|s| s.miner_pubkey == "miner_c").unwrap();
        assert_eq!(miner_c.share_pct, 25.0);
        assert_eq!(miner_c.reward_amount, 250);
    }

    #[test]
    fn test_pplns_empty_proofs() {
        let strategy = PPLNSPool {
            window_size: 3,
            min_stake: 0,
        };
        let proofs = vec![];

        let shares = strategy.calculate_shares(&proofs, 1000).unwrap();

        assert_eq!(shares.len(), 0);
    }

    #[test]
    fn test_pplns_window_larger_than_proofs() {
        let strategy = PPLNSPool {
            window_size: 10,
            min_stake: 0,
        };
        let proofs = vec![
            create_proof("miner_a"),
            create_proof("miner_b"),
            create_proof("miner_a"),
        ];

        let shares = strategy.calculate_shares(&proofs, 1000).unwrap();

        // Window size larger than proofs, use all proofs
        // miner_a: 2/3 = ~66.67%
        // miner_b: 1/3 = ~33.33%

        assert_eq!(shares.len(), 2);

        // Total must equal reward
        let total: u64 = shares.iter().map(|s| s.reward_amount).sum();
        assert_eq!(total, 1000);

        let miner_a = shares.iter().find(|s| s.miner_pubkey == "miner_a").unwrap();
        assert!((miner_a.share_pct - 66.67).abs() < 0.1);
        assert!(miner_a.reward_amount >= 666 && miner_a.reward_amount <= 667);

        let miner_b = shares.iter().find(|s| s.miner_pubkey == "miner_b").unwrap();
        assert!((miner_b.share_pct - 33.33).abs() < 0.1);
        assert!(miner_b.reward_amount >= 333 && miner_b.reward_amount <= 334);
    }

    // =========================================================================
    // Miner Acceptance Tests
    // =========================================================================

    #[test]
    fn test_proportional_accept_miner_with_sufficient_stake() {
        let strategy = ProportionalPool { min_stake: 100 };
        let miner = create_miner("miner_a", 150);

        let accepted = strategy.accept_miner(&miner).unwrap();

        assert!(accepted);
    }

    #[test]
    fn test_proportional_reject_miner_with_insufficient_stake() {
        let strategy = ProportionalPool { min_stake: 100 };
        let miner = create_miner("miner_a", 50);

        let accepted = strategy.accept_miner(&miner).unwrap();

        assert!(!accepted);
    }

    #[test]
    fn test_pplns_accept_miner_with_sufficient_stake() {
        let strategy = PPLNSPool {
            window_size: 5,
            min_stake: 200,
        };
        let miner = create_miner("miner_a", 300);

        let accepted = strategy.accept_miner(&miner).unwrap();

        assert!(accepted);
    }

    #[test]
    fn test_pplns_reject_miner_with_insufficient_stake() {
        let strategy = PPLNSPool {
            window_size: 5,
            min_stake: 200,
        };
        let miner = create_miner("miner_a", 100);

        let accepted = strategy.accept_miner(&miner).unwrap();

        assert!(!accepted);
    }

    // =========================================================================
    // Pool Info Tests
    // =========================================================================

    #[test]
    fn test_proportional_pool_info() {
        let strategy = ProportionalPool { min_stake: 50 };

        let info = strategy.pool_info(5, 500);

        assert_eq!(info.pool_type, "proportional");
        assert_eq!(info.total_miners, 5);
        assert_eq!(info.total_stake, 500);
        assert_eq!(info.minimum_stake, 50);
    }

    #[test]
    fn test_pplns_pool_info() {
        let strategy = PPLNSPool {
            window_size: 10,
            min_stake: 100,
        };

        let info = strategy.pool_info(3, 300);

        assert_eq!(info.pool_type, "pplns");
        assert_eq!(info.total_miners, 3);
        assert_eq!(info.total_stake, 300);
        assert_eq!(info.minimum_stake, 100);
    }

    // =========================================================================
    // SoloStrategy Tests (regression)
    // =========================================================================

    #[test]
    fn test_solo_pool_type() {
        let strategy = SoloStrategy;
        assert_eq!(strategy.pool_type(), PoolType::Solo);
    }

    #[test]
    fn test_solo_minimum_stake() {
        let strategy = SoloStrategy;
        assert_eq!(strategy.minimum_stake(), 0);
    }

    #[test]
    fn test_solo_accept_miner() {
        let strategy = SoloStrategy;
        let miner = create_miner("miner_a", 0);
        assert!(strategy.accept_miner(&miner).unwrap());
    }

    #[test]
    fn test_solo_calculate_shares() {
        let strategy = SoloStrategy;
        let proofs = vec![
            create_proof("miner_a"),
            create_proof("miner_b"),
            create_proof("miner_c"),
        ];

        let shares = strategy.calculate_shares(&proofs, 1000).unwrap();

        // Solo strategy gives all to the last proof submitter
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].miner_pubkey, "miner_c");
        assert_eq!(shares[0].reward_amount, 1000);
    }

    #[test]
    fn test_solo_empty_proofs() {
        let strategy = SoloStrategy;
        let proofs = vec![];

        let shares = strategy.calculate_shares(&proofs, 1000).unwrap();

        assert_eq!(shares.len(), 0);
    }
}
