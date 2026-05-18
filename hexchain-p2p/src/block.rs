use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::lattice_geometry::{get_neighbors, HCPCoord};
use crate::types::NEIGHBOR_SLOT_EMPTY;
use crate::types::{is_empty_neighbor_slot, BlockHash, TensorMeta, HASH_BYTES, NEIGHBOR_SLOTS};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HexBlock {
    pub parent_hash: BlockHash,
    pub tx_merkle_root: BlockHash,
    pub timestamp: u64,
    pub nonce: u64,
    pub coord: HCPCoord,
    pub neighbor_hashes: [BlockHash; NEIGHBOR_SLOTS],
    pub tensor: TensorMeta,
}

pub fn sha256_once(data: &[u8]) -> BlockHash {
    let mut h = Sha256::new();
    h.update(data);
    let result = h.finalize();
    let mut out = [0u8; HASH_BYTES];
    out.copy_from_slice(&result);
    out
}

pub fn sha256_pair(a: &BlockHash, b: &BlockHash) -> BlockHash {
    let mut buf = [0u8; HASH_BYTES * 2];
    buf[..HASH_BYTES].copy_from_slice(a);
    buf[HASH_BYTES..].copy_from_slice(b);
    sha256_once(&buf)
}

fn sha256_double_pair(a: &BlockHash, b: &BlockHash) -> BlockHash {
    let h1 = sha256_pair(a, b);
    sha256_once(&h1)
}

pub fn merkle_root_neighbors(leaves: &[BlockHash; NEIGHBOR_SLOTS]) -> BlockHash {
    let mut level: Vec<BlockHash> = leaves.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for chunk in level.chunks(2) {
            if chunk.len() == 2 {
                next.push(sha256_double_pair(&chunk[0], &chunk[1]));
            } else {
                next.push(sha256_double_pair(&chunk[0], &chunk[0]));
            }
        }
        level = next;
    }
    level[0]
}

impl HexBlock {
    pub fn neighbor_merkle_root(&self) -> BlockHash {
        merkle_root_neighbors(&self.neighbor_hashes)
    }

    pub fn serialize_pow_preimage(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32 + 32 + 8 + 8 + 4 + 4 + 4 + 32 + 8 + 8 + 8 + 8);

        out.extend_from_slice(&self.parent_hash);
        out.extend_from_slice(&self.tx_merkle_root);
        out.extend_from_slice(&self.timestamp.to_le_bytes());
        out.extend_from_slice(&self.nonce.to_le_bytes());
        out.extend_from_slice(&(self.coord.q as u32).to_le_bytes());
        out.extend_from_slice(&(self.coord.r as u32).to_le_bytes());
        out.extend_from_slice(&(self.coord.s as u32).to_le_bytes());
        let nm = self.neighbor_merkle_root();
        out.extend_from_slice(&nm);
        out.extend_from_slice(&self.tensor.expected_capacity.to_le_bytes());
        out.extend_from_slice(&self.tensor.actual_capacity.to_le_bytes());
        out.extend_from_slice(&self.tensor.compression_num.to_le_bytes());
        out.extend_from_slice(&self.tensor.compression_den.to_le_bytes());

        out
    }

    pub fn pow_hash(&self) -> BlockHash {
        let preimage = self.serialize_pow_preimage();
        sha256_once(&preimage)
    }
}

pub fn fill_neighbor_slots_from_store<F>(
    coord: HCPCoord,
    mut lookup: F,
) -> [BlockHash; NEIGHBOR_SLOTS]
where
    F: FnMut(HCPCoord) -> Option<BlockHash>,
{
    let nb = get_neighbors(coord);
    let mut hashes = [NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS];
    for i in 0..NEIGHBOR_SLOTS {
        if let Some(h) = lookup(nb[i]) {
            hashes[i] = h;
        }
    }
    hashes
}

pub fn count_nonempty_neighbors(block: &HexBlock) -> usize {
    block
        .neighbor_hashes
        .iter()
        .filter(|h| !is_empty_neighbor_slot(h))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_once_deterministic() {
        let data = b"hello hexchain";
        let a = sha256_once(data);
        let b = sha256_once(data);
        assert_eq!(a, b);
    }

    #[test]
    fn test_sha256_once_different_inputs() {
        let a = sha256_once(b"input a");
        let b = sha256_once(b"input b");
        assert_ne!(a, b);
    }

    #[test]
    fn test_merkle_root_all_empty() {
        let leaves = [NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS];
        let root = merkle_root_neighbors(&leaves);
        // 12 zeros → 6 pairs → 3 pairs → 2 elements (odd pairs w/ self) → 1 root
        let p = sha256_double_pair(&[0u8; 32], &[0u8; 32]);
        let pp = sha256_double_pair(&p, &p);
        let ppp = sha256_double_pair(&pp, &pp);
        let expected = sha256_double_pair(&ppp, &ppp);
        assert_eq!(root, expected);
    }

    #[test]
    fn test_pow_hash_deterministic() {
        let block = HexBlock {
            parent_hash: [1u8; 32],
            tx_merkle_root: [2u8; 32],
            timestamp: 100,
            nonce: 42,
            coord: HCPCoord { q: 0, r: 0, s: 0 },
            neighbor_hashes: [NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS],
            tensor: TensorMeta {
                expected_capacity: 1000,
                actual_capacity: 1000,
                compression_num: 1,
                compression_den: 1,
            },
        };
        let h1 = block.pow_hash();
        let h2 = block.pow_hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_pow_hash_changes_with_nonce() {
        let mut block = HexBlock {
            parent_hash: [1u8; 32],
            tx_merkle_root: [2u8; 32],
            timestamp: 100,
            nonce: 0,
            coord: HCPCoord { q: 0, r: 0, s: 0 },
            neighbor_hashes: [NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS],
            tensor: TensorMeta {
                expected_capacity: 1000,
                actual_capacity: 1000,
                compression_num: 1,
                compression_den: 1,
            },
        };
        let h0 = block.pow_hash();
        block.nonce = 1;
        let h1 = block.pow_hash();
        assert_ne!(h0, h1);
    }

    #[test]
    fn test_pow_hash_changes_with_coord() {
        let mut block = HexBlock {
            parent_hash: [0u8; 32],
            tx_merkle_root: [0u8; 32],
            timestamp: 0,
            nonce: 0,
            coord: HCPCoord { q: 0, r: 0, s: 0 },
            neighbor_hashes: [NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS],
            tensor: TensorMeta::default(),
        };
        let h0 = block.pow_hash();
        block.coord = HCPCoord { q: 1, r: 0, s: 0 };
        let h1 = block.pow_hash();
        assert_ne!(h0, h1);
    }

    #[test]
    fn test_neighbor_merkle_root_filled_vs_empty() {
        let empty_block = HexBlock {
            parent_hash: [0u8; 32],
            tx_merkle_root: [0u8; 32],
            timestamp: 0,
            nonce: 0,
            coord: HCPCoord { q: 0, r: 0, s: 0 },
            neighbor_hashes: [NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS],
            tensor: TensorMeta::default(),
        };

        let mut filled_hashes = [NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS];
        filled_hashes[0] = [1u8; 32];
        let filled_block = HexBlock {
            neighbor_hashes: filled_hashes,
            ..empty_block.clone()
        };

        assert_ne!(
            empty_block.neighbor_merkle_root(),
            filled_block.neighbor_merkle_root()
        );
    }

    #[test]
    fn test_serialize_preimage_length() {
        let block = HexBlock {
            parent_hash: [0u8; 32],
            tx_merkle_root: [0u8; 32],
            timestamp: 0,
            nonce: 0,
            coord: HCPCoord { q: 0, r: 0, s: 0 },
            neighbor_hashes: [NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS],
            tensor: TensorMeta::default(),
        };
        let preimage = block.serialize_pow_preimage();
        assert_eq!(
            preimage.len(),
            32 + 32 + 8 + 8 + 4 + 4 + 4 + 32 + 8 + 8 + 8 + 8
        );
    }

    #[test]
    fn test_count_nonempty_all_empty() {
        let block = HexBlock {
            parent_hash: [0u8; 32],
            tx_merkle_root: [0u8; 32],
            timestamp: 0,
            nonce: 0,
            coord: HCPCoord { q: 0, r: 0, s: 0 },
            neighbor_hashes: [NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS],
            tensor: TensorMeta::default(),
        };
        assert_eq!(count_nonempty_neighbors(&block), 0);
    }

    #[test]
    fn test_count_nonempty_some() {
        let mut hashes = [NEIGHBOR_SLOT_EMPTY; NEIGHBOR_SLOTS];
        hashes[0] = [1u8; 32];
        hashes[3] = [2u8; 32];
        hashes[11] = [3u8; 32];
        let block = HexBlock {
            parent_hash: [0u8; 32],
            tx_merkle_root: [0u8; 32],
            timestamp: 0,
            nonce: 0,
            coord: HCPCoord { q: 0, r: 0, s: 0 },
            neighbor_hashes: hashes,
            tensor: TensorMeta::default(),
        };
        assert_eq!(count_nonempty_neighbors(&block), 3);
    }

    #[test]
    fn test_fill_neighbor_slots_from_store() {
        let coord = HCPCoord { q: 0, r: 0, s: 0 };
        let mut store = std::collections::HashMap::new();
        store.insert(HCPCoord { q: 1, r: 0, s: 0 }, [0xAAu8; 32]);
        store.insert(HCPCoord { q: 0, r: 0, s: 1 }, [0xBBu8; 32]);

        let hashes = fill_neighbor_slots_from_store(coord, |c| store.get(&c).copied());
        // index 0 = (1,0,0) planar offset, which IS stored
        assert_eq!(hashes[0], [0xAAu8; 32]);
        // index 6 = (0,0,1) upper offset, which IS stored
        assert_eq!(hashes[6], [0xBBu8; 32]);
        // index 1 = (1,-1,0) should be empty
        assert!(is_empty_neighbor_slot(&hashes[1]));
    }
}
