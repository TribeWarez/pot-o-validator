use crate::internal_api::PeerInfo;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PeerStore {
    peers: Arc<RwLock<Vec<PeerInfo>>>,
    path: String,
}

impl PeerStore {
    pub fn new(path: String, peers: Arc<RwLock<Vec<PeerInfo>>>) -> Self {
        Self { peers, path }
    }

    pub fn load(&self) {
        if let Ok(content) = std::fs::read_to_string(&self.path) {
            if let Ok(peers) = serde_json::from_str::<Vec<PeerInfo>>(&content) {
                let mut current = self.peers.blocking_write();
                *current = peers;
            }
        }
    }

    pub async fn save(&self) -> Result<(), String> {
        let peers = self.peers.read().await.clone();
        let json = serde_json::to_string_pretty(&peers).map_err(|e| e.to_string())?;
        let tmp = format!("{}.tmp", self.path);
        std::fs::write(&tmp, &json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.path).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn spawn_persist(&self) {
        let peers = self.peers.clone();
        let path = self.path.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let current = peers.read().await.clone();
                let json = match serde_json::to_string_pretty(&current) {
                    Ok(j) => j,
                    Err(_) => continue,
                };
                let tmp = format!("{}.tmp", path);
                let _ = std::fs::write(&tmp, &json);
                let _ = std::fs::rename(&tmp, &path);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_peer_store_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("peers.json");
        let peers = Arc::new(RwLock::new(vec![]));

        let store = PeerStore::new(path.to_str().unwrap().to_string(), peers.clone());

        peers.blocking_write().push(PeerInfo {
            node_id: "test".to_string(),
            url: "http://localhost:8900".to_string(),
            last_seen: Utc::now(),
        });

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(store.save()).unwrap();

        let peers2 = Arc::new(RwLock::new(vec![]));
        let store2 = PeerStore::new(path.to_str().unwrap().to_string(), peers2.clone());
        store2.load();

        assert_eq!(peers2.blocking_read().len(), 1);
        assert_eq!(peers2.blocking_read()[0].node_id, "test");
    }

    #[test]
    fn test_peer_store_load_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");
        let peers = Arc::new(RwLock::new(vec![]));

        let store = PeerStore::new(path.to_str().unwrap().to_string(), peers.clone());
        store.load();

        assert_eq!(peers.blocking_read().len(), 0);
    }

    #[test]
    fn test_peer_store_load_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("peers.json");
        std::fs::write(&path, "not valid json").unwrap();
        let peers = Arc::new(RwLock::new(vec![]));

        let store = PeerStore::new(path.to_str().unwrap().to_string(), peers.clone());
        store.load();

        assert_eq!(peers.blocking_read().len(), 0);
    }

    #[test]
    fn test_peer_store_atomic_write_no_tmp_left() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("peers.json");
        let peers = Arc::new(RwLock::new(vec![]));

        let store = PeerStore::new(path.to_str().unwrap().to_string(), peers.clone());

        peers.blocking_write().push(PeerInfo {
            node_id: "v1".to_string(),
            url: "http://v1:8900".to_string(),
            last_seen: Utc::now(),
        });

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(store.save()).unwrap();

        assert!(path.exists());
        let tmp_path = format!("{}.tmp", path.display());
        assert!(!std::path::Path::new(&tmp_path).exists());
    }
}
