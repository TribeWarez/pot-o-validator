use crate::consensus::{calculate_target, count_mature_neighbors};
use crate::lattice_geometry::get_neighbors;
use crate::lattice_store::LatticeStore;
use crate::types::{is_empty_neighbor_slot, ConsensusParams, TensorMeta, ValidationError, NEIGHBOR_SLOTS};
use crate::uint256::Uint256;
use crate::block::HexBlock;

pub fn validate_mml(tensor: &TensorMeta, max_num: u64, max_den: u64) -> Option<ValidationError> {
    if tensor.compression_den == 0 || max_den == 0 {
        return Some(ValidationError::MmlExceeded);
    }
    let lhs = (tensor.compression_num as u128) * (max_den as u128);
    let rhs = (max_num as u128) * (tensor.compression_den as u128);
    if lhs <= rhs {
        None
    } else {
        Some(ValidationError::MmlExceeded)
    }
}

pub fn validate_block(
    block: &HexBlock,
    store: &LatticeStore,
    params: &ConsensusParams,
) -> Option<ValidationError> {
    let neighbors = get_neighbors(block.coord);
    for i in 0..NEIGHBOR_SLOTS {
        let nb = neighbors[i];
        let claimed = block.neighbor_hashes[i];
        let at_lattice = store.hash_at(nb);

        if is_empty_neighbor_slot(&claimed) {
            if at_lattice.is_some() {
                return Some(ValidationError::LatticeMismatch);
            }
        } else {
            match at_lattice {
                Some(h) if h == claimed => {}
                _ => return Some(ValidationError::LatticeMismatch),
            }
        }
    }

    let k = count_mature_neighbors(&block.neighbor_hashes, params.maturity_depth, |h| {
        store.depth_of(h)
    });
    if k == 0 {
        return Some(ValidationError::NoMatureNeighbors);
    }

    if let Some(err) = validate_mml(&block.tensor, params.mml.max_num, params.mml.max_den) {
        return Some(err);
    }

    let base = Uint256::from_be_bytes(params.base_target);
    let target_eff = calculate_target(&base, k, params.symmetry_num, params.symmetry_den);
    let hv = Uint256::from_be_bytes(block.pow_hash());

    if hv <= target_eff {
        None
    } else {
        Some(ValidationError::PowTooHigh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{fill_neighbor_slots_from_store, HexBlock};
    use crate::lattice_geometry::HCPCoord;
    use crate::types::{MmlParams, NEIGHBOR_SLOT_EMPTY, TensorMeta};

    fn make_genesis_block() -> HexBlock {
        HexBlock {
            parent_hash: [0u8; 32],
            tx_merkle_root: [0u8; 32],
            timestamp: 1,
            nonce: 0,
            coord: HCPCoord { q: 0, r: 0, s: 0 },
            neighbor_hashes: [NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS],
            tensor: TensorMeta {
                expected_capacity: 1000,
                actual_capacity: 1000,
                compression_num: 1,
                compression_den: 1,
            },
        }
    }

    fn demo_params() -> ConsensusParams {
        ConsensusParams {
            maturity_depth: 10,
            symmetry_num: 115,
            symmetry_den: 100,
            base_target: [0xFFu8; 32],
            mml: MmlParams {
                max_num: 100,
                max_den: 100,
            },
        }
    }

    #[test]
    fn test_validate_mml_ok() {
        let tensor = TensorMeta {
            compression_num: 95,
            compression_den: 100,
            ..TensorMeta::default()
        };
        assert_eq!(validate_mml(&tensor, 95, 100), None);
    }

    #[test]
    fn test_validate_mml_exact_boundary() {
        let tensor = TensorMeta {
            compression_num: 95,
            compression_den: 100,
            ..TensorMeta::default()
        };
        assert_eq!(validate_mml(&tensor, 95, 100), None);
    }

    #[test]
    fn test_validate_mml_exceeded() {
        let tensor = TensorMeta {
            compression_num: 96,
            compression_den: 100,
            ..TensorMeta::default()
        };
        assert_eq!(
            validate_mml(&tensor, 95, 100),
            Some(ValidationError::MmlExceeded)
        );
    }

    #[test]
    fn test_validate_mml_den_zero() {
        let tensor = TensorMeta {
            compression_num: 1,
            compression_den: 0,
            ..TensorMeta::default()
        };
        assert_eq!(
            validate_mml(&tensor, 95, 100),
            Some(ValidationError::MmlExceeded)
        );
    }

    #[test]
    fn test_validate_mml_max_den_zero() {
        let tensor = TensorMeta {
            compression_num: 1,
            compression_den: 100,
            ..TensorMeta::default()
        };
        assert_eq!(
            validate_mml(&tensor, 95, 0),
            Some(ValidationError::MmlExceeded)
        );
    }

    #[test]
    fn test_genesis_fails_no_mature_neighbors() {
        let store = LatticeStore::new();
        let block = make_genesis_block();
        let params = demo_params();

        let result = validate_block(&block, &store, &params);
        assert_eq!(result, Some(ValidationError::NoMatureNeighbors));
    }

    #[test]
    fn test_validate_block_lattice_mismatch_claimed_nonempty_but_empty() {
        let store = LatticeStore::new();
        let mut block = make_genesis_block();
        block.neighbor_hashes[0] = [0x01u8; 32]; // claim non-empty but store has nothing

        let params = demo_params();
        let result = validate_block(&block, &store, &params);
        assert_eq!(result, Some(ValidationError::LatticeMismatch));
    }

    #[test]
    fn test_demo_workflow_valid_block() {
        let store = LatticeStore::new();
        let params = demo_params();

        let genesis = make_genesis_block();
        let genesis_hash = genesis.pow_hash();
        store.insert(genesis.coord, genesis_hash, 11);

        let rim_coord = HCPCoord { q: 1, r: 0, s: 0 };
        let neighbor_hashes = fill_neighbor_slots_from_store(rim_coord, |c| store.hash_at(c));

        let mut block = HexBlock {
            parent_hash: [0u8; 32],
            tx_merkle_root: [0u8; 32],
            timestamp: 2,
            nonce: 0,
            coord: rim_coord,
            neighbor_hashes,
            tensor: TensorMeta {
                expected_capacity: 1250,
                actual_capacity: 1250,
                compression_num: 95,
                compression_den: 100,
            },
        };

        let k = count_mature_neighbors(
            &block.neighbor_hashes,
            params.maturity_depth,
            |h| store.depth_of(h),
        );

        let base = Uint256::from_be_bytes(params.base_target);
        let target_eff = calculate_target(&base, k, params.symmetry_num, params.symmetry_den);

        block.nonce = 0;
        loop {
            let hv = Uint256::from_be_bytes(block.pow_hash());
            if hv <= target_eff {
                break;
            }
            block.nonce += 1;
        }

        let result = validate_block(&block, &store, &params);
        assert_eq!(result, None, "valid rim block should pass validation");
    }

    #[test]
    fn test_block_at_rim_passes_with_mature_neighbor() {
        let store = LatticeStore::new();
        let params = ConsensusParams {
            maturity_depth: 0,
            ..demo_params()
        };

        // Insert genesis at (0,0,0) with depth 1
        let genesis = make_genesis_block();
        let genesis_hash = genesis.pow_hash();
        store.insert(HCPCoord { q: 0, r: 0, s: 0 }, genesis_hash, 1);

        // Build a block at (1,0,0). Genesis is at offset (-1,0,0), which is planar slot 3.
        let mut hashes = [NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS];
        hashes[3] = genesis_hash;

        let block = HexBlock {
            parent_hash: [0u8; 32],
            tx_merkle_root: [0u8; 32],
            timestamp: 2,
            nonce: 0,
            coord: HCPCoord { q: 1, r: 0, s: 0 },
            neighbor_hashes: hashes,
            tensor: TensorMeta {
                expected_capacity: 1000,
                actual_capacity: 1000,
                compression_num: 1,
                compression_den: 1,
            },
        };

        let result = validate_block(&block, &store, &params);
        assert_eq!(result, None, "block with mature neighbor should pass");
    }

    #[test]
    fn test_pow_too_high() {
        let store = LatticeStore::new();
        // Insert genesis at (0,0,0) so we can place a block that has a mature neighbor
        let genesis = make_genesis_block();
        let genesis_hash = genesis.pow_hash();
        store.insert(HCPCoord { q: 0, r: 0, s: 0 }, genesis_hash, 1);

        // Very small target: only PoW hashes <= 0x10 are valid
        let params = ConsensusParams {
            maturity_depth: 0,
            base_target: [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                          0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                          0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                          0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10],
            ..demo_params()
        };

        // Block at (1,0,0) referencing genesis at slot 3 = (-1,0,0) offset
        let mut hashes = [NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS];
        hashes[3] = genesis_hash;

        let block = HexBlock {
            parent_hash: [0u8; 32],
            tx_merkle_root: [0u8; 32],
            timestamp: 2,
            nonce: 0,
            coord: HCPCoord { q: 1, r: 0, s: 0 },
            neighbor_hashes: hashes,
            tensor: TensorMeta {
                expected_capacity: 1000,
                actual_capacity: 1000,
                compression_num: 1,
                compression_den: 1,
            },
        };

        let result = validate_block(&block, &store, &params);
        assert_eq!(result, Some(ValidationError::PowTooHigh),
            "PoW should be too high for extremely small target");
    }

    #[test]
    fn test_lattice_mismatch_claimed_empty_but_store_has() {
        let store = LatticeStore::new();
        store.insert(HCPCoord { q: 1, r: 0, s: 0 }, [0x01u8; 32], 10);

        let block = HexBlock {
            neighbor_hashes: [NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS],
            ..make_genesis_block()
        };

        let params = ConsensusParams {
            maturity_depth: 0,
            ..demo_params()
        };

        let result = validate_block(&block, &store, &params);
        assert_eq!(result, Some(ValidationError::LatticeMismatch));
    }

    #[test]
    fn test_lattice_mismatch_wrong_hash() {
        let store = LatticeStore::new();
        store.insert(HCPCoord { q: 1, r: 0, s: 0 }, [0x01u8; 32], 10);

        let mut hashes = [NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS];
        hashes[0] = [0x02u8; 32]; // wrong hash

        let block = HexBlock {
            neighbor_hashes: hashes,
            ..make_genesis_block()
        };

        let params = ConsensusParams {
            maturity_depth: 0,
            ..demo_params()
        };
        store.insert(block.coord, block.pow_hash(), 1);

        let result = validate_block(&block, &store, &params);
        assert_eq!(result, Some(ValidationError::LatticeMismatch));
    }
}
