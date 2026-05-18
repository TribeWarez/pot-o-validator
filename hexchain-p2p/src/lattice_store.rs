use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use crate::lattice_geometry::HCPCoord;
use crate::types::BlockHash;

pub struct LatticeStore {
    coord_to_hash: RwLock<HashMap<HCPCoord, BlockHash>>,
    hash_to_depth: RwLock<HashMap<BlockHash, u64>>,
}

impl LatticeStore {
    pub fn new() -> Self {
        Self {
            coord_to_hash: RwLock::new(HashMap::new()),
            hash_to_depth: RwLock::new(HashMap::new()),
        }
    }

    pub fn hash_at(&self, coord: HCPCoord) -> Option<BlockHash> {
        let map = self.coord_to_hash.read().unwrap();
        map.get(&coord).copied()
    }

    pub fn insert(&self, coord: HCPCoord, hash: BlockHash, depth: u64) {
        {
            let mut map = self.coord_to_hash.write().unwrap();
            map.insert(coord, hash);
        }
        {
            let mut map = self.hash_to_depth.write().unwrap();
            map.insert(hash, depth);
        }
    }

    pub fn depth_of(&self, hash: &BlockHash) -> Option<u64> {
        let map = self.hash_to_depth.read().unwrap();
        map.get(hash).copied()
    }

    pub fn contains_coord(&self, coord: HCPCoord) -> bool {
        let map = self.coord_to_hash.read().unwrap();
        map.contains_key(&coord)
    }

    pub fn is_empty(&self) -> bool {
        let map = self.coord_to_hash.read().unwrap();
        map.is_empty()
    }

    pub fn all_coords(&self) -> HashSet<HCPCoord> {
        let map = self.coord_to_hash.read().unwrap();
        map.keys().copied().collect()
    }

    pub fn all_blocks(&self) -> Vec<(HCPCoord, BlockHash)> {
        let map = self.coord_to_hash.read().unwrap();
        map.iter().map(|(c, h)| (*c, *h)).collect()
    }
}

impl Default for LatticeStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_store() {
        let store = LatticeStore::new();
        assert_eq!(store.hash_at(HCPCoord { q: 0, r: 0, s: 0 }), None);
        assert_eq!(store.depth_of(&[0u8; 32]), None);
        assert!(!store.contains_coord(HCPCoord { q: 0, r: 0, s: 0 }));
    }

    #[test]
    fn test_insert_and_retrieve() {
        let store = LatticeStore::new();
        let coord = HCPCoord { q: 5, r: -3, s: 2 };
        let hash = [0xABu8; 32];
        let depth = 42;

        store.insert(coord, hash, depth);

        assert_eq!(store.hash_at(coord), Some(hash));
        assert!(store.contains_coord(coord));
    }

    #[test]
    fn test_depth_of() {
        let store = LatticeStore::new();
        let hash = [0xCDu8; 32];

        store.insert(HCPCoord { q: 1, r: 0, s: 0 }, hash, 100);
        assert_eq!(store.depth_of(&hash), Some(100));
    }

    #[test]
    fn test_overwrite_coord() {
        let store = LatticeStore::new();
        let coord = HCPCoord { q: 0, r: 0, s: 0 };

        store.insert(coord, [0xAAu8; 32], 10);
        store.insert(coord, [0xBBu8; 32], 20);

        assert_eq!(store.hash_at(coord), Some([0xBBu8; 32]));
        assert_eq!(store.depth_of(&[0xBBu8; 32]), Some(20));
    }

    #[test]
    fn test_depth_of_nonexistent() {
        let store = LatticeStore::new();
        assert_eq!(store.depth_of(&[0xFFu8; 32]), None);
    }

    #[test]
    fn test_multiple_inserts() {
        let store = LatticeStore::new();
        store.insert(HCPCoord { q: 1, r: 0, s: 0 }, [1u8; 32], 5);
        store.insert(HCPCoord { q: 0, r: 1, s: 0 }, [2u8; 32], 10);
        store.insert(HCPCoord { q: 0, r: 0, s: 1 }, [3u8; 32], 15);

        assert_eq!(
            store.hash_at(HCPCoord { q: 1, r: 0, s: 0 }),
            Some([1u8; 32])
        );
        assert_eq!(
            store.hash_at(HCPCoord { q: 0, r: 1, s: 0 }),
            Some([2u8; 32])
        );
        assert_eq!(
            store.hash_at(HCPCoord { q: 0, r: 0, s: 1 }),
            Some([3u8; 32])
        );
    }

    #[test]
    fn test_concurrent_access() {
        let store = std::sync::Arc::new(LatticeStore::new());
        let mut handles = vec![];

        for i in 0..8 {
            let s = store.clone();
            handles.push(std::thread::spawn(move || {
                let coord = HCPCoord { q: i, r: 0, s: 0 };
                let hash = [i as u8; 32];
                s.insert(coord, hash, i as u64);
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        for i in 0..8 {
            let coord = HCPCoord { q: i, r: 0, s: 0 };
            let hash = [i as u8; 32];
            assert_eq!(store.hash_at(coord), Some(hash));
            assert_eq!(store.depth_of(&hash), Some(i as u64));
        }
    }
}
