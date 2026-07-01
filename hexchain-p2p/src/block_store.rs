use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use std::sync::RwLock;

use crate::types::BlockHash;

/// Full block data stored for each block (including transactions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredBlock {
    pub hash: BlockHash,
    pub height: u64,
    pub block_json: String,
}

/// Thread-safe store for full block data
pub struct BlockStore {
    blocks: RwLock<HashMap<[u8; 32], StoredBlock>>,
    by_height: RwLock<BTreeMap<u64, [u8; 32]>>,
    path: RwLock<String>,
    modified: RwLock<bool>,
}

impl BlockStore {
    /// Constructor, loads from file if exists.
    pub fn new(path: &str) -> Self {
        let store = Self {
            blocks: RwLock::new(HashMap::new()),
            by_height: RwLock::new(BTreeMap::new()),
            path: RwLock::new(path.to_string()),
            modified: RwLock::new(false),
        };
        let _ = store.load_from_file();
        store
    }

    /// Store a block indexed by hash and height.
    pub fn insert(&self, hash: &[u8; 32], height: u64, block_json: &str) {
        let block = StoredBlock {
            hash: *hash,
            height,
            block_json: block_json.to_string(),
        };
        {
            let mut blocks = self.blocks.write().unwrap();
            blocks.insert(*hash, block);
        }
        {
            let mut by_height = self.by_height.write().unwrap();
            by_height.insert(height, *hash);
        }
        {
            let mut modified = self.modified.write().unwrap();
            *modified = true;
        }
    }

    /// Retrieve a block by its hash.
    pub fn get(&self, hash: &[u8; 32]) -> Option<StoredBlock> {
        let blocks = self.blocks.read().unwrap();
        blocks.get(hash).cloned()
    }

    /// Retrieve a block by its height.
    pub fn at_height(&self, height: u64) -> Option<StoredBlock> {
        let by_height = self.by_height.read().unwrap();
        let hash = by_height.get(&height).copied()?;
        drop(by_height);
        self.get(&hash)
    }

    /// Highest height in the store.
    pub fn latest_height(&self) -> u64 {
        let by_height = self.by_height.read().unwrap();
        by_height.keys().last().copied().unwrap_or(0)
    }

    /// Block at the highest height.
    pub fn latest_block(&self) -> Option<StoredBlock> {
        let height = self.latest_height();
        self.at_height(height)
    }

    /// Append a full HexBlock to the store.
    pub fn append(&self, block: &crate::block::HexBlock) {
        let hash = block.pow_hash();
        let block_json = serde_json::to_string(block).unwrap_or_default();
        self.insert(&hash, block.height, &block_json);
    }

    /// Check if a block with the given hash exists.
    pub fn has(&self, hash: &[u8; 32]) -> bool {
        let blocks = self.blocks.read().unwrap();
        blocks.contains_key(hash)
    }

    /// Dirty flag indicating whether blocks have been inserted since last clear.
    pub fn is_modified(&self) -> bool {
        let modified = self.modified.read().unwrap();
        *modified
    }

    /// Reset the dirty flag.
    pub fn clear_modified(&self) {
        let mut modified = self.modified.write().unwrap();
        *modified = false;
    }

    /// Load blocks from a JSON file. Silently returns Err if file doesn't exist.
    fn load_from_file(&self) -> Result<(), String> {
        let path = self.path.read().unwrap().clone();
        if !Path::new(&path).exists() {
            return Err("file not found".into());
        }
        let json = fs::read_to_string(&path).map_err(|e| format!("read error: {}", e))?;
        let stored_blocks: Vec<StoredBlock> =
            serde_json::from_str(&json).map_err(|e| format!("parse error: {}", e))?;
        let mut blocks = self.blocks.write().unwrap();
        let mut by_height = self.by_height.write().unwrap();
        for block in stored_blocks {
            let hash = block.hash;
            let height = block.height;
            blocks.insert(hash, block);
            by_height.insert(height, hash);
        }
        Ok(())
    }

    /// Save blocks to a JSON file atomically (.tmp then rename).
    pub fn save_to_file(&self) -> Result<(), String> {
        let path = self.path.read().unwrap().clone();
        let blocks = self.blocks.read().unwrap();
        let stored_blocks: Vec<StoredBlock> = blocks.values().cloned().collect();
        drop(blocks);
        let json = serde_json::to_string_pretty(&stored_blocks)
            .map_err(|e| format!("serialization error: {}", e))?;
        let tmp = format!("{}.tmp", path);
        fs::write(&tmp, &json).map_err(|e| format!("write error: {}", e))?;
        fs::rename(&tmp, &path).map_err(|e| format!("rename error: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_store_insert_and_retrieve() {
        let dir = std::env::temp_dir();
        let path = dir
            .join("test_blockstore.json")
            .to_string_lossy()
            .to_string();
        let store = BlockStore::new(&path);

        let hash = [1u8; 32];
        store.insert(&hash, 0, r#"{"height":0}"#);
        assert!(store.has(&hash));
        assert_eq!(store.latest_height(), 0);
        assert_eq!(store.at_height(0).unwrap().height, 0);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_block_store_persistence() {
        let dir = std::env::temp_dir();
        let path = dir
            .join("test_blockstore_persist.json")
            .to_string_lossy()
            .to_string();

        {
            let store = BlockStore::new(&path);
            store.insert(&[2u8; 32], 1, r#"{"height":1}"#);
            store.save_to_file().unwrap();
        }
        {
            let store = BlockStore::new(&path);
            assert!(store.has(&[2u8; 32]));
            assert_eq!(store.latest_height(), 1);
        }

        let _ = std::fs::remove_file(&path);
    }
}
