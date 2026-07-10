use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::RwLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::lattice_geometry::HCPCoord;
use crate::types::BlockHash;

/// Default path for the lattice state JSON file.
pub const DEFAULT_LATTICE_PATH: &str = "hexchain_lattice.json";

/// Serializable snapshot of the lattice state (RwLock cannot derive serde).
/// Hash map keys are hex-encoded for JSON compatibility.
#[derive(Debug, Clone)]
pub struct LatticeSnapshot {
    pub coord_to_hash: HashMap<HCPCoord, BlockHash>,
    pub hash_to_depth: HashMap<BlockHash, u64>,
}

/// Helper: encode a byte slice as a hex string.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Helper: decode a hex string into a fixed-size array.
fn hex_decode<const N: usize>(s: &str) -> Result<[u8; N], String> {
    let bytes: Vec<u8> = (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("hex decode error: {}", e))?;
    if bytes.len() != N {
        return Err(format!("expected {} bytes, got {}", N, bytes.len()));
    }
    let mut arr = [0u8; N];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

/// Helper: encode HCPCoord as "q,r,s".
fn coord_encode(c: &HCPCoord) -> String {
    format!("{},{},{}", c.q, c.r, c.s)
}

/// Helper: decode "q,r,s" into HCPCoord.
fn coord_decode(s: &str) -> Result<HCPCoord, String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 3 {
        return Err("expected 3 comma-separated ints".into());
    }
    let q = parts[0]
        .parse::<i32>()
        .map_err(|e| format!("bad q: {}", e))?;
    let r = parts[1]
        .parse::<i32>()
        .map_err(|e| format!("bad r: {}", e))?;
    let s = parts[2]
        .parse::<i32>()
        .map_err(|e| format!("bad s: {}", e))?;
    Ok(HCPCoord { q, r, s })
}

impl Serialize for LatticeSnapshot {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("LatticeSnapshot", 2)?;
        let coord_pairs: Vec<(String, String)> = self
            .coord_to_hash
            .iter()
            .map(|(c, h)| (coord_encode(c), hex_encode(h)))
            .collect();
        state.serialize_field("coord_to_hash", &coord_pairs)?;
        let depth_pairs: Vec<(String, u64)> = self
            .hash_to_depth
            .iter()
            .map(|(k, v)| (hex_encode(k), *v))
            .collect();
        state.serialize_field("hash_to_depth", &depth_pairs)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for LatticeSnapshot {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            CoordToHash,
            HashToDepth,
        }

        struct SnapshotVisitor;

        impl<'de> Visitor<'de> for SnapshotVisitor {
            type Value = LatticeSnapshot;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct LatticeSnapshot")
            }

            fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<LatticeSnapshot, V::Error> {
                let mut coord_to_hash = None;
                let mut hash_to_depth = None;
                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::CoordToHash => {
                            if coord_to_hash.is_some() {
                                return Err(de::Error::duplicate_field("coord_to_hash"));
                            }
                            let pairs: Vec<(String, String)> = map.next_value()?;
                            let mut cth = HashMap::with_capacity(pairs.len());
                            for (coord_str, hash_hex) in pairs {
                                let coord = coord_decode(&coord_str).map_err(de::Error::custom)?;
                                let hash =
                                    hex_decode::<32>(&hash_hex).map_err(de::Error::custom)?;
                                cth.insert(coord, hash);
                            }
                            coord_to_hash = Some(cth);
                        }
                        Field::HashToDepth => {
                            if hash_to_depth.is_some() {
                                return Err(de::Error::duplicate_field("hash_to_depth"));
                            }
                            let pairs: Vec<(String, u64)> = map.next_value()?;
                            let mut htd = HashMap::with_capacity(pairs.len());
                            for (hex_str, depth) in pairs {
                                let hash = hex_decode::<32>(&hex_str).map_err(de::Error::custom)?;
                                htd.insert(hash, depth);
                            }
                            hash_to_depth = Some(htd);
                        }
                    }
                }
                let coord_to_hash =
                    coord_to_hash.ok_or_else(|| de::Error::missing_field("coord_to_hash"))?;
                let hash_to_depth =
                    hash_to_depth.ok_or_else(|| de::Error::missing_field("hash_to_depth"))?;
                Ok(LatticeSnapshot {
                    coord_to_hash,
                    hash_to_depth,
                })
            }
        }

        const FIELDS: &[&str] = &["coord_to_hash", "hash_to_depth"];
        deserializer.deserialize_struct("LatticeSnapshot", FIELDS, SnapshotVisitor)
    }
}

pub struct LatticeStore {
    coord_to_hash: RwLock<HashMap<HCPCoord, BlockHash>>,
    hash_to_depth: RwLock<HashMap<BlockHash, u64>>,
    timestamps: RwLock<Vec<u64>>,
    path: RwLock<String>,
}

impl LatticeStore {
    pub fn new() -> Self {
        Self {
            coord_to_hash: RwLock::new(HashMap::new()),
            hash_to_depth: RwLock::new(HashMap::new()),
            timestamps: RwLock::new(Vec::new()),
            path: RwLock::new(DEFAULT_LATTICE_PATH.to_string()),
        }
    }

    /// Create a store with a specific persistence path.
    pub fn with_path(path: &str) -> Self {
        let store = Self::new();
        *store.path.write().unwrap() = path.to_string();
        store
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

    pub fn record_timestamp(&self, timestamp: u64) {
        let mut ts = self.timestamps.write().unwrap();
        ts.push(timestamp);
    }

    pub fn recent_timestamps(&self, limit: usize) -> Vec<u64> {
        let ts = self.timestamps.read().unwrap();
        let start = ts.len().saturating_sub(limit);
        ts[start..].to_vec()
    }

    /// Take a consistent snapshot of the full lattice state.
    pub fn snapshot(&self) -> LatticeSnapshot {
        let coord_to_hash = self.coord_to_hash.read().unwrap().clone();
        let hash_to_depth = self.hash_to_depth.read().unwrap().clone();
        LatticeSnapshot {
            coord_to_hash,
            hash_to_depth,
        }
    }

    /// Load a snapshot into this store (merges, keeping deeper entries on conflict).
    pub fn merge_snapshot(&self, snapshot: &LatticeSnapshot) {
        let mut coord_to_hash = self.coord_to_hash.write().unwrap();
        let mut hash_to_depth = self.hash_to_depth.write().unwrap();
        for (coord, hash) in &snapshot.coord_to_hash {
            let current_depth = coord_to_hash
                .get(coord)
                .and_then(|h| hash_to_depth.get(h))
                .copied();
            let incoming_depth = snapshot.hash_to_depth.get(hash).copied();
            match (current_depth, incoming_depth) {
                (Some(cur), Some(inc)) if inc > cur => {
                    coord_to_hash.insert(*coord, *hash);
                    hash_to_depth.insert(*hash, inc);
                }
                (None, Some(_)) => {
                    coord_to_hash.insert(*coord, *hash);
                    hash_to_depth.insert(*hash, incoming_depth.unwrap_or(0));
                }
                _ => {}
            }
        }
    }

    // ── File persistence ────────────────────────────────────────────────

    /// Save current lattice state to a JSON file atomically.
    pub fn save_to_file(&self) -> Result<(), String> {
        let path = self.path.read().unwrap().clone();
        let snapshot = self.snapshot();
        let json = serde_json::to_string_pretty(&snapshot)
            .map_err(|e| format!("serialization error: {}", e))?;
        let tmp = format!("{}.tmp", path);
        fs::write(&tmp, &json).map_err(|e| format!("write error: {}", e))?;
        fs::rename(&tmp, &path).map_err(|e| format!("rename error: {}", e))?;
        Ok(())
    }

    /// Load lattice state from a JSON file. Returns Ok with number of entries loaded,
    /// or Err if file doesn't exist or is unreadable.
    pub fn load_from_file(&self) -> Result<usize, String> {
        let path = self.path.read().unwrap().clone();
        if !Path::new(&path).exists() {
            return Err("file not found".into());
        }
        let json = fs::read_to_string(&path).map_err(|e| format!("read error: {}", e))?;
        let snapshot: LatticeSnapshot =
            serde_json::from_str(&json).map_err(|e| format!("parse error: {}", e))?;
        let count = snapshot.coord_to_hash.len();
        self.merge_snapshot(&snapshot);
        Ok(count)
    }

    /// Spawn a background thread that persists the lattice every `interval`.
    pub fn spawn_persist(self: &std::sync::Arc<Self>, interval: Duration) {
        let store = self.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(interval);
            if let Err(e) = store.save_to_file() {
                eprintln!("[lattice] persist error: {}", e);
            }
        });
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

    // ── Persistence tests ───────────────────────────────────────────────

    #[test]
    fn test_snapshot_roundtrip() {
        let store = LatticeStore::new();
        store.insert(HCPCoord { q: 0, r: 0, s: 0 }, [1u8; 32], 10);
        store.insert(HCPCoord { q: 1, r: 0, s: 0 }, [2u8; 32], 20);

        let snap = store.snapshot();
        assert_eq!(snap.coord_to_hash.len(), 2);
        assert_eq!(snap.hash_to_depth[&[1u8; 32]], 10);
        assert_eq!(snap.hash_to_depth[&[2u8; 32]], 20);
    }

    #[test]
    fn test_save_and_load() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp
            .path()
            .join("test_lattice.json")
            .to_str()
            .unwrap()
            .to_string();

        let store = LatticeStore::with_path(&path);
        store.insert(HCPCoord { q: 3, r: -1, s: 2 }, [0xABu8; 32], 42);
        store.insert(HCPCoord { q: 0, r: 0, s: 1 }, [0xCDu8; 32], 7);
        store.save_to_file().unwrap();

        let loaded = LatticeStore::with_path(&path);
        let count = loaded.load_from_file().unwrap();
        assert_eq!(count, 2);
        assert_eq!(
            loaded.hash_at(HCPCoord { q: 3, r: -1, s: 2 }),
            Some([0xABu8; 32])
        );
        assert_eq!(loaded.depth_of(&[0xCDu8; 32]), Some(7));
    }

    #[test]
    fn test_merge_snapshot_deeper_wins() {
        let store = LatticeStore::new();
        store.insert(HCPCoord { q: 0, r: 0, s: 0 }, [1u8; 32], 5);

        let mut incoming = LatticeSnapshot {
            coord_to_hash: HashMap::new(),
            hash_to_depth: HashMap::new(),
        };
        incoming
            .coord_to_hash
            .insert(HCPCoord { q: 0, r: 0, s: 0 }, [2u8; 32]);
        incoming
            .coord_to_hash
            .insert(HCPCoord { q: 1, r: 0, s: 0 }, [3u8; 32]);
        incoming.hash_to_depth.insert([2u8; 32], 10); // deeper
        incoming.hash_to_depth.insert([3u8; 32], 1);

        store.merge_snapshot(&incoming);

        // (0,0,0) should have taken [2u8;32] (depth 10 > 5)
        assert_eq!(
            store.hash_at(HCPCoord { q: 0, r: 0, s: 0 }),
            Some([2u8; 32])
        );
        // (1,0,0) should have [3u8;32] (new coord)
        assert_eq!(
            store.hash_at(HCPCoord { q: 1, r: 0, s: 0 }),
            Some([3u8; 32])
        );
    }

    #[test]
    fn test_merge_snapshot_keeps_deeper_existing() {
        let store = LatticeStore::new();
        store.insert(HCPCoord { q: 0, r: 0, s: 0 }, [1u8; 32], 20); // existing depth 20

        let mut incoming = LatticeSnapshot {
            coord_to_hash: HashMap::new(),
            hash_to_depth: HashMap::new(),
        };
        incoming
            .coord_to_hash
            .insert(HCPCoord { q: 0, r: 0, s: 0 }, [2u8; 32]);
        incoming.hash_to_depth.insert([2u8; 32], 5); // shallower

        store.merge_snapshot(&incoming);

        // Should keep [1u8;32] since depth 20 > 5
        assert_eq!(
            store.hash_at(HCPCoord { q: 0, r: 0, s: 0 }),
            Some([1u8; 32])
        );
    }

    #[test]
    fn test_load_missing_file() {
        let store = LatticeStore::with_path("/tmp/nonexistent_lattice_file_xyz.json");
        let result = store.load_from_file();
        assert!(result.is_err());
    }
}
