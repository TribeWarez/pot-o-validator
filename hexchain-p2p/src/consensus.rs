use crate::types::{BlockHash, NEIGHBOR_SLOTS};
use crate::uint256::Uint256;

/// Deterministic conflict resolution: given two competing entries for the same
/// lattice coordinate, pick the winner.
///
/// Rules (in order of priority):
/// 1. Higher chain depth wins
/// 2. If depths are equal, higher subgraph weight (cumulative PoW) wins
/// 3. If both are equal, lexicographically larger hash wins (deterministic tiebreak)
pub fn resolve_conflict(
    cur_hash: BlockHash,
    cur_depth: u64,
    cur_weight: u64,
    inc_hash: BlockHash,
    inc_depth: u64,
    inc_weight: u64,
) -> (BlockHash, u64, u64) {
    if inc_depth > cur_depth {
        (inc_hash, inc_depth, inc_weight)
    } else if inc_depth < cur_depth {
        (cur_hash, cur_depth, cur_weight)
    } else if inc_weight > cur_weight {
        (inc_hash, inc_depth, inc_weight)
    } else if inc_weight < cur_weight {
        (cur_hash, cur_depth, cur_weight)
    } else if inc_hash > cur_hash {
        (inc_hash, inc_depth, inc_weight)
    } else {
        (cur_hash, cur_depth, cur_weight)
    }
}

pub fn calculate_target(
    base_target: &Uint256,
    mature_neighbors: usize,
    symmetry_num: u64,
    symmetry_den: u64,
) -> Uint256 {
    let k = mature_neighbors.clamp(0, NEIGHBOR_SLOTS);
    let mut t = *base_target;
    let sn = symmetry_num as u32;
    let sd = if symmetry_den == 0 {
        1u32
    } else {
        symmetry_den as u32
    };
    for _ in 0..k {
        t.mul_div(sn, sd);
    }
    t
}

pub fn count_mature_neighbors<F>(
    neighbor_hashes: &[BlockHash; NEIGHBOR_SLOTS],
    maturity_depth: u64,
    depth_of: F,
) -> usize
where
    F: Fn(&BlockHash) -> Option<u64>,
{
    neighbor_hashes
        .iter()
        .filter(|h| !crate::types::is_empty_neighbor_slot(h))
        .filter_map(depth_of)
        .filter(|&depth| depth > maturity_depth)
        .count()
}

pub fn subgraph_weight_stub_local_pow(hash_below_target_bits: u64) -> u64 {
    hash_below_target_bits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uint256::Uint256;

    #[test]
    fn test_resolve_conflict_deeper_wins() {
        let cur = [1u8; 32];
        let inc = [2u8; 32];
        let (hash, depth, _) = resolve_conflict(cur, 5, 100, inc, 10, 50);
        assert_eq!(hash, inc, "deeper incoming should win");
        assert_eq!(depth, 10);
    }

    #[test]
    fn test_resolve_conflict_shallower_loses() {
        let cur = [1u8; 32];
        let inc = [2u8; 32];
        let (hash, depth, _) = resolve_conflict(cur, 10, 100, inc, 5, 200);
        assert_eq!(hash, cur, "shallower incoming should lose");
        assert_eq!(depth, 10);
    }

    #[test]
    fn test_resolve_conflict_equal_depth_higher_weight_wins() {
        let cur = [1u8; 32];
        let inc = [2u8; 32];
        let (hash, _, _) = resolve_conflict(cur, 10, 100, inc, 10, 200);
        assert_eq!(hash, inc, "higher weight at same depth should win");
    }

    #[test]
    fn test_resolve_conflict_equal_depth_lower_weight_loses() {
        let cur = [1u8; 32];
        let inc = [2u8; 32];
        let (hash, _, _) = resolve_conflict(cur, 10, 200, inc, 10, 100);
        assert_eq!(hash, cur, "lower weight at same depth should lose");
    }

    #[test]
    fn test_resolve_conflict_equal_depth_equal_weight_larger_hash_wins() {
        let small_hash = [0x01u8; 32];
        let large_hash = [0x02u8; 32];
        let (hash, _, _) = resolve_conflict(small_hash, 10, 100, large_hash, 10, 100);
        assert_eq!(
            hash, large_hash,
            "equal depth+weight: larger hash should win"
        );
    }

    #[test]
    fn test_resolve_conflict_equal_depth_equal_weight_smaller_hash_loses() {
        let small_hash = [0x01u8; 32];
        let large_hash = [0x02u8; 32];
        let (hash, _, _) = resolve_conflict(large_hash, 10, 100, small_hash, 10, 100);
        assert_eq!(
            hash, large_hash,
            "equal depth+weight: incoming smaller hash should lose"
        );
    }

    #[test]
    fn test_calculate_target_k_zero_returns_base() {
        let base = Uint256::from_u64(1000);
        let result = calculate_target(&base, 0, 115, 100);
        assert_eq!(result, base);
    }

    #[test]
    fn test_calculate_target_increases_with_k() {
        let base = Uint256::from_u64(1000);
        let t0 = calculate_target(&base, 0, 115, 100);
        let t1 = calculate_target(&base, 1, 115, 100);
        let t12 = calculate_target(&base, 12, 115, 100);
        assert!(t1 > t0);
        assert!(t12 > t1);
    }

    #[test]
    fn test_calculate_target_k_clamped() {
        let base = Uint256::from_u64(1000);
        let t12 = calculate_target(&base, 12, 115, 100);
        let t20 = calculate_target(&base, 20, 115, 100);
        assert_eq!(t12, t20, "k should be clamped to 12");
    }

    #[test]
    fn test_calculate_target_symmetry_den_zero_guard() {
        let base = Uint256::from_u64(1000);
        let t = calculate_target(&base, 1, 115, 0);
        let expected = {
            let mut v = base;
            v.mul_div(115, 1);
            v
        };
        assert_eq!(
            t,
            expected,
            "den=0 should be treated as 1, so 1000*115/1 = {}",
            expected.as_be_bytes()[31]
        );
    }

    #[test]
    fn test_calculate_target_decreases_with_den_gt_num() {
        let base = Uint256::from_u64(1000);
        let t = calculate_target(&base, 3, 85, 100);
        assert!(t < base);
    }

    #[test]
    fn test_count_mature_neighbors_all_empty() {
        let hashes = [crate::types::NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS];
        let count = count_mature_neighbors(&hashes, 10, |_| Some(100));
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_mature_neighbors_counts_deep_enough() {
        let mut hashes = [crate::types::NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS];
        hashes[0] = [1u8; 32];
        hashes[1] = [2u8; 32];
        hashes[2] = [3u8; 32];

        let count = count_mature_neighbors(&hashes, 10, |h| match h[0] {
            1 => Some(50),
            2 => Some(5),
            3 => Some(20),
            _ => None,
        });
        assert_eq!(
            count, 2,
            "should count hashes[0] and hashes[2] (depth 50 and 20 > 10)"
        );
    }

    #[test]
    fn test_count_mature_neighbors_skips_none() {
        let mut hashes = [crate::types::NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS];
        hashes[0] = [1u8; 32];
        hashes[1] = [2u8; 32];

        let count =
            count_mature_neighbors(&hashes, 10, |h| if h[0] == 1 { Some(50) } else { None });
        assert_eq!(count, 1, "should skip hash[1] since depth_of returns None");
    }

    #[test]
    fn test_subgraph_weight_stub() {
        assert_eq!(subgraph_weight_stub_local_pow(42), 42);
        assert_eq!(subgraph_weight_stub_local_pow(0), 0);
    }
}
