use std::sync::Arc;

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::block::{fill_neighbor_slots_from_store, HexBlock};
use crate::consensus::{calculate_target, count_mature_neighbors};
use crate::lattice_geometry::{get_neighbors, HCPCoord};
use crate::lattice_store::LatticeStore;
use crate::types::{BlockHash, ConsensusParams, ValidationError, NEIGHBOR_SLOTS};
use crate::uint256::Uint256;
use crate::validator::validate_block;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexChallenge {
    pub id: String,
    pub slot: u64,
    pub slot_hash: String,
    pub coord: HCPCoord,
    pub target: BlockHash,
    pub consensus_params: ConsensusParams,
    pub neighbor_hashes: [BlockHash; NEIGHBOR_SLOTS],
    pub created_at_unix: u64,
    pub expires_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexProof {
    pub challenge_id: String,
    pub block: HexBlock,
    pub miner_pubkey: String,
    pub timestamp_unix: u64,
}

pub struct HexConsensus {
    pub params: ConsensusParams,
    pub store: Arc<LatticeStore>,
}

impl HexConsensus {
    pub fn new(params: ConsensusParams) -> Self {
        Self {
            params,
            store: Arc::new(LatticeStore::new()),
        }
    }

    pub fn generate_challenge(&self, slot: u64, slot_hash: &str) -> HexChallenge {
        let coord = self.pick_coord();
        let neighbor_hashes = fill_neighbor_slots_from_store(coord, |c| self.store.hash_at(c));
        let k = count_mature_neighbors(&neighbor_hashes, self.params.maturity_depth, |h| {
            self.store.depth_of(h)
        });
        let base = Uint256::from_be_bytes(self.params.base_target);
        let target = calculate_target(&base, k, self.params.symmetry_num, self.params.symmetry_den);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let id = format!("hex_{}_{}", slot, slot_hash.get(..8).unwrap_or(slot_hash));

        HexChallenge {
            id,
            slot,
            slot_hash: slot_hash.to_string(),
            coord,
            target: *target.as_be_bytes(),
            consensus_params: self.params,
            neighbor_hashes,
            created_at_unix: now,
            expires_at_unix: now + 120,
        }
    }

    pub fn mine(
        &self,
        challenge: &HexChallenge,
        max_iterations: u64,
        transactions: Option<Vec<serde_json::Value>>,
        height: u64,
        miner_address: Option<String>,
    ) -> Option<HexProof> {
        let target = Uint256::from_be_bytes(challenge.target);

        for nonce in 0..max_iterations {
            let block = HexBlock {
                parent_hash: challenge.neighbor_hashes[0],
                height,
                tx_merkle_root: compute_tx_merkle_root(transactions.as_deref()),
                transactions: transactions.clone(),
                miner_address: miner_address.clone(),
                timestamp: challenge.created_at_unix,
                nonce,
                coord: challenge.coord,
                neighbor_hashes: challenge.neighbor_hashes,
                tensor: crate::types::TensorMeta {
                    expected_capacity: 1000,
                    actual_capacity: 1000,
                    compression_num: 95,
                    compression_den: 100,
                },
            };

            let hv = Uint256::from_be_bytes(block.pow_hash());
            if hv <= target {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                return Some(HexProof {
                    challenge_id: challenge.id.clone(),
                    block,
                    miner_pubkey: String::new(),
                    timestamp_unix: now,
                });
            }
        }

        None
    }

    pub fn verify_proof(&self, proof: &HexProof) -> Result<bool, ValidationError> {
        if self.store.is_empty() {
            return Ok(true);
        }
        match validate_block(&proof.block, &self.store, &self.params) {
            None => Ok(true),
            Some(e) => Err(e),
        }
    }

    pub fn submit_block(&self, proof: &HexProof) -> Result<u64, ValidationError> {
        let genesis_mode = self.store.is_empty();

        if !genesis_mode {
            self.verify_proof(proof)?;
        }

        let new_depth = if genesis_mode {
            self.params.maturity_depth + 1
        } else {
            let max_depth = self
                .store
                .all_blocks()
                .iter()
                .filter_map(|(_, h)| self.store.depth_of(h))
                .max()
                .unwrap_or(0);
            max_depth + 1
        };

        self.store
            .insert(proof.block.coord, proof.block.pow_hash(), new_depth);

        Ok(new_depth)
    }

    fn pick_coord(&self) -> HCPCoord {
        let occupied = self.store.all_coords();
        if occupied.is_empty() {
            return HCPCoord { q: 0, r: 0, s: 0 };
        }

        let mut frontier_set = std::collections::BTreeSet::new();
        for &coord in &occupied {
            for nb in get_neighbors(coord) {
                if !occupied.contains(&nb) {
                    frontier_set.insert(nb);
                }
            }
        }

        if frontier_set.is_empty() {
            return HCPCoord { q: 0, r: 0, s: 0 };
        }

        let frontier: Vec<HCPCoord> = frontier_set.into_iter().collect();
        let mut rng = rand::thread_rng();
        frontier
            .choose(&mut rng)
            .copied()
            .unwrap_or(HCPCoord { q: 0, r: 0, s: 0 })
    }
}

/// Compute a merkle root over a list of serialized transactions.
/// Uses single SHA-256 (must match the validation function in `consensus.rs`).
fn compute_tx_merkle_root(txs: Option<&[serde_json::Value]>) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let leaves: Vec<[u8; 32]> = match txs {
        Some(tx_list) if !tx_list.is_empty() => tx_list
            .iter()
            .map(|tx| {
                let data = serde_json::to_string(tx).unwrap_or_else(|_| String::new());
                let mut hasher = Sha256::new();
                hasher.update(data.as_bytes());
                let result = hasher.finalize();
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&result);
                hash
            })
            .collect(),
        _ => return [0u8; 32],
    };

    let mut level = leaves;
    while level.len() > 1 {
        let mut next = Vec::new();
        for chunk in level.chunks(2) {
            let mut hasher = Sha256::new();
            hasher.update(chunk[0]);
            hasher.update(if chunk.len() > 1 { chunk[1] } else { chunk[0] });
            let result = hasher.finalize();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&result);
            next.push(hash);
        }
        level = next;
    }
    level[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ConsensusParams;

    fn default_hex_consensus() -> HexConsensus {
        HexConsensus::new(ConsensusParams::default())
    }

    #[test]
    fn test_generate_challenge_empty_lattice() {
        let hc = default_hex_consensus();
        let chal = hc.generate_challenge(0, "genesis_slot");
        assert_eq!(chal.coord, HCPCoord { q: 0, r: 0, s: 0 });
        assert_eq!(chal.target, [0xFFu8; 32]);
    }

    #[test]
    fn test_generate_challenge_after_genesis() {
        let hc = default_hex_consensus();

        let chal = hc.generate_challenge(0, "slot0");
        let proof = hc
            .mine(&chal, 100_000, None, 0, None)
            .expect("should mine genesis");
        hc.submit_block(&proof).expect("genesis should submit");

        let chal2 = hc.generate_challenge(1, "slot1");
        assert_ne!(chal2.coord, HCPCoord { q: 0, r: 0, s: 0 });
        let frontier_coord = chal2.coord;
        let nbs = get_neighbors(HCPCoord { q: 0, r: 0, s: 0 });
        assert!(
            nbs.contains(&frontier_coord),
            "challenge coord should be neighbor of genesis"
        );
    }

    #[test]
    fn test_mine_and_submit_lifecycle() {
        let hc = default_hex_consensus();
        let chal = hc.generate_challenge(0, "lifecycle");
        let proof = hc
            .mine(&chal, 100_000, None, 0, None)
            .expect("should find proof");

        let verify_result = hc.verify_proof(&proof);
        assert!(
            verify_result.is_ok(),
            "proof should verify: {:?}",
            verify_result
        );

        let depth = hc.submit_block(&proof).expect("submit should succeed");
        assert_eq!(
            depth,
            hc.params.maturity_depth + 1,
            "genesis depth = maturity_depth + 1"
        );
        assert!(hc.store.contains_coord(proof.block.coord));
    }

    #[test]
    fn test_challenge_coord_is_random_frontier() {
        let hc = default_hex_consensus();
        let chal = hc.generate_challenge(0, "s0");
        let proof = hc.mine(&chal, 100_000, None, 0, None).unwrap();
        hc.submit_block(&proof).unwrap();

        let chal2 = hc.generate_challenge(1, "s1");
        let proof2 = hc.mine(&chal2, 100_000, None, 1, None).unwrap();
        hc.submit_block(&proof2).unwrap();

        let occupied = hc.store.all_coords();
        assert_eq!(occupied.len(), 2);
        let nbs0 = get_neighbors(HCPCoord { q: 0, r: 0, s: 0 });
        let nbs1 = get_neighbors(proof.block.coord);
        assert!(nbs0.contains(&proof2.block.coord) || nbs1.contains(&proof2.block.coord));
    }

    #[test]
    fn test_verify_rejects_tampered_block() {
        let hc = default_hex_consensus();
        let chal = hc.generate_challenge(0, "g");
        let proof = hc.mine(&chal, 1000, None, 0, None).expect("genesis");
        hc.submit_block(&proof).expect("submit genesis");

        // Build a proof with an invalid block referencing a nonexistent neighbor hash
        let bad_hash = [0xABu8; 32];
        let mut bad_hashes = [crate::types::NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS];
        bad_hashes[0] = bad_hash;
        let bad_block = HexBlock {
            parent_hash: [0u8; 32],
            tx_merkle_root: [0u8; 32],
            height: 0,
            transactions: None,
            miner_address: None,
            timestamp: 0,
            nonce: 0,
            coord: HCPCoord { q: 99, r: 0, s: 0 },
            neighbor_hashes: bad_hashes,
            tensor: crate::types::TensorMeta {
                expected_capacity: 1000,
                actual_capacity: 1000,
                compression_num: 95,
                compression_den: 100,
            },
        };
        let bad_proof = HexProof {
            challenge_id: "bad".into(),
            block: bad_block,
            miner_pubkey: String::new(),
            timestamp_unix: 0,
        };

        let verify_result = hc.verify_proof(&bad_proof);
        assert!(
            verify_result.is_err(),
            "bad proof should fail: {:?}",
            verify_result
        );
    }

    #[test]
    fn test_multiple_blocks_chain() {
        let hc = default_hex_consensus();

        for i in 0..3 {
            let chal = hc.generate_challenge(i, &format!("slot_{}", i));
            let proof = hc
                .mine(&chal, 200_000, None, i as u64, None)
                .expect(&format!("should mine block {}", i));
            hc.submit_block(&proof)
                .expect(&format!("should submit block {}", i));
        }

        assert_eq!(hc.store.all_coords().len(), 3);
    }
}
