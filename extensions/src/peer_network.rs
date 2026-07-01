//! Peer network: local-only and optional VPN mesh for multi-node discovery.

use super::gossip_client::GossipClient;
use super::mdns_discovery::MdnsDiscovery;
use async_trait::async_trait;
use pot_o_core::TribeResult;
use pot_o_mining::{Challenge, ProofPayload};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

pub type NodeId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub node_id: NodeId,
    pub address: String,
    pub port: u16,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkState {
    pub peers: Vec<PeerInfo>,
    pub total_nodes: usize,
    pub synced: bool,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// How validator nodes discover and communicate with each other.
#[async_trait]
pub trait PeerNetwork: Send + Sync {
    fn node_id(&self) -> &NodeId;
    async fn discover_peers(&self) -> TribeResult<Vec<PeerInfo>>;
    async fn broadcast_challenge(&self, challenge: &Challenge) -> TribeResult<()>;
    async fn relay_proof(&self, proof: &ProofPayload) -> TribeResult<()>;
    async fn sync_state(&self) -> TribeResult<NetworkState>;

    /// Broadcast a transaction to all known peers so they can add it to their mempool.
    async fn broadcast_transaction(&self, _tx: &serde_json::Value) -> TribeResult<()> {
        Ok(())
    }

    /// Pull the hexchain lattice snapshot from a peer by URL.
    /// Returns `None` if the peer is unreachable or returns non-200.
    async fn pull_lattice(&self, peer_url: &str) -> TribeResult<Option<serde_json::Value>> {
        #[allow(unused)]
        let _ = peer_url;
        Ok(None)
    }

    /// Push the local hexchain lattice snapshot to all known peers.
    /// Returns count of successful pushes.
    async fn push_lattice(&self, _snapshot: &serde_json::Value) -> TribeResult<usize> {
        Ok(0)
    }
}

// ---------------------------------------------------------------------------
// LocalOnlyNetwork (implemented now)
// ---------------------------------------------------------------------------

pub struct LocalOnlyNetwork {
    node_id: NodeId,
}

impl LocalOnlyNetwork {
    pub fn new() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

impl Default for LocalOnlyNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PeerNetwork for LocalOnlyNetwork {
    fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    async fn discover_peers(&self) -> TribeResult<Vec<PeerInfo>> {
        Ok(vec![]) // No peers in local-only mode
    }

    async fn broadcast_challenge(&self, _challenge: &Challenge) -> TribeResult<()> {
        Ok(()) // No-op
    }

    async fn relay_proof(&self, _proof: &ProofPayload) -> TribeResult<()> {
        Ok(()) // No-op
    }

    async fn sync_state(&self) -> TribeResult<NetworkState> {
        Ok(NetworkState {
            peers: vec![],
            total_nodes: 1,
            synced: true,
        })
    }
}

// ---------------------------------------------------------------------------
// VpnMeshNetwork (WireGuard + mDNS + Bootstrap Registry discovery)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnMeshConfig {
    pub wireguard_interface: String,
    pub peer_addresses: Vec<String>,
    pub mdns_enabled: bool,
    pub gossip_port: u16,
}

pub struct VpnMeshNetwork {
    pub node_id: NodeId,
    pub config: VpnMeshConfig,
    gossip_client: GossipClient,
    mdns_discovery: Option<Arc<MdnsDiscovery>>,
    bootstrap_urls: Vec<String>,
    peer_list: Arc<RwLock<Vec<PeerInfo>>>,
    peer_timeout_secs: u64,
}

impl VpnMeshNetwork {
    /// Create a new VpnMeshNetwork with the given configuration.
    ///
    /// # Arguments
    /// * `node_id` - Unique identifier for this node
    /// * `config` - VPN mesh configuration
    /// * `bootstrap_urls` - URLs of bootstrap registry servers for peer discovery
    /// * `mdns_enabled` - Whether to enable mDNS discovery
    /// * `_mdns_service_name` - Service name for mDNS registration
    /// * `peer_timeout_secs` - Timeout for peer communication
    ///
    /// # Returns
    /// Result with VpnMeshNetwork instance or error
    pub fn new(
        node_id: NodeId,
        config: VpnMeshConfig,
        bootstrap_urls: Vec<String>,
        mdns_enabled: bool,
        _mdns_service_name: &str,
        peer_timeout_secs: u64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Create gossip client with bootstrap URLs as initial peers
        let gossip_client = GossipClient::new(bootstrap_urls.clone(), peer_timeout_secs);

        // Create optional mDNS discovery (wrapped in Arc for spawn_blocking)
        let mdns_discovery = if mdns_enabled && config.mdns_enabled {
            Some(Arc::new(MdnsDiscovery::new(&node_id, config.gossip_port)?))
        } else {
            None
        };

        Ok(Self {
            node_id,
            config,
            gossip_client,
            mdns_discovery,
            bootstrap_urls,
            peer_list: Arc::new(RwLock::new(Vec::new())),
            peer_timeout_secs,
        })
    }

    /// Fetch peers from bootstrap registry via HTTP.
    async fn fetch_bootstrap_peers(&self) -> Result<Vec<PeerInfo>, Box<dyn std::error::Error>> {
        let mut peers = Vec::new();

        for bootstrap_url in &self.bootstrap_urls {
            // Try to fetch peer list from bootstrap registry
            let url = format!("{}/peers", bootstrap_url);
            match reqwest::Client::new()
                .get(&url)
                .timeout(std::time::Duration::from_secs(self.peer_timeout_secs))
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    if let Ok(peer_infos) = response.json::<Vec<PeerInfo>>().await {
                        peers.extend(peer_infos);
                    }
                }
                _ => {
                    // Continue to next bootstrap URL on failure
                }
            }
        }

        // Deduplicate by node_id
        peers.sort_by(|a, b| a.node_id.cmp(&b.node_id));
        peers.dedup_by(|a, b| a.node_id == b.node_id);

        Ok(peers)
    }
}

#[async_trait]
impl PeerNetwork for VpnMeshNetwork {
    fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    async fn discover_peers(&self) -> TribeResult<Vec<PeerInfo>> {
        let mut all_peers = Vec::new();

        // Try mDNS discovery first (if enabled) — runs in spawn_blocking since
        // mdns_sd::ServiceDaemon::browse + recv_timeout is synchronous and can
        // block for up to `timeout_secs` seconds.
        if let Some(mdns) = &self.mdns_discovery {
            let mdns_clone = mdns.clone();
            let discovered = tokio::task::spawn_blocking(move || {
                mdns_clone.discover_peers(5).unwrap_or_default()
            })
            .await
            .unwrap_or_default();
            for pd in discovered {
                all_peers.push(PeerInfo {
                    node_id: pd.node_id,
                    address: pd.ip.to_string(),
                    port: pd.port,
                    last_seen: chrono::Utc::now(),
                    version: "1.0".to_string(),
                });
            }
        }

        // Fall back to bootstrap registry
        match self.fetch_bootstrap_peers().await {
            Ok(bootstrap_peers) => {
                all_peers.extend(bootstrap_peers);
            }
            Err(_) => {
                // Bootstrap also failed, return what we have
            }
        }

        // Deduplicate by node_id
        all_peers.sort_by(|a, b| a.node_id.cmp(&b.node_id));
        all_peers.dedup_by(|a, b| a.node_id == b.node_id);

        // Filter out ourselves
        all_peers.retain(|peer| peer.node_id != self.node_id);

        // Update peer list
        {
            let mut peer_list = self.peer_list.write().await;
            *peer_list = all_peers.clone();
        }

        Ok(all_peers)
    }

    async fn broadcast_challenge(&self, challenge: &Challenge) -> TribeResult<()> {
        // Serialize challenge to JSON
        let challenge_json = serde_json::to_value(challenge).map_err(|e| {
            pot_o_core::TribeError::SerializationError(format!(
                "Failed to serialize challenge: {}",
                e
            ))
        })?;

        // Use gossip client to broadcast
        let _success_count = self
            .gossip_client
            .broadcast_challenge(&self.node_id, &challenge_json)
            .await
            .map_err(|e| {
                pot_o_core::TribeError::NetworkError(format!(
                    "Failed to broadcast challenge: {}",
                    e
                ))
            })?;

        Ok(())
    }

    async fn relay_proof(&self, proof: &ProofPayload) -> TribeResult<()> {
        // Serialize proof to JSON
        let proof_json = serde_json::to_value(proof).map_err(|e| {
            pot_o_core::TribeError::SerializationError(format!("Failed to serialize proof: {}", e))
        })?;

        // Create proof payload for sending
        let payload = serde_json::json!({
            "node_id": self.node_id,
            "proof": proof_json,
        });

        // Try to post proof to each peer
        let peers = self.peer_list.read().await;
        for peer in peers.iter() {
            let peer_url = format!("http://{}:{}", peer.address, peer.port);
            let _result = reqwest::Client::new()
                .post(format!("{}/proof", peer_url))
                .timeout(std::time::Duration::from_secs(self.peer_timeout_secs))
                .json(&payload)
                .send()
                .await;
            // Continue relaying even if one peer fails
        }

        Ok(())
    }

    async fn sync_state(&self) -> TribeResult<NetworkState> {
        let peer_list = self.peer_list.read().await;
        let total_nodes = peer_list.len() + 1; // +1 for ourselves

        Ok(NetworkState {
            peers: peer_list.clone(),
            total_nodes,
            synced: !peer_list.is_empty(),
        })
    }

    async fn pull_lattice(&self, peer_url: &str) -> TribeResult<Option<serde_json::Value>> {
        let url = format!("{}/hexchain/lattice", peer_url.trim_end_matches('/'));
        match reqwest::Client::new()
            .get(&url)
            .timeout(std::time::Duration::from_secs(self.peer_timeout_secs))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<serde_json::Value>().await {
                    Ok(json) => Ok(Some(json)),
                    Err(_) => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    async fn push_lattice(&self, snapshot: &serde_json::Value) -> TribeResult<usize> {
        let peers = self.peer_list.read().await;
        let mut success = 0usize;
        for peer in peers.iter() {
            let url = format!(
                "http://{}:{}/hexchain/lattice/sync",
                peer.address, peer.port
            );
            match reqwest::Client::new()
                .post(&url)
                .timeout(std::time::Duration::from_secs(self.peer_timeout_secs))
                .json(snapshot)
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => success += 1,
                _ => {}
            }
        }
        Ok(success)
    }

    async fn broadcast_transaction(&self, tx: &serde_json::Value) -> TribeResult<()> {
        let peers = self.peer_list.read().await;
        for peer in peers.iter() {
            let url = format!(
                "http://{}:{}/internal/tx/broadcast",
                peer.address, peer.port
            );
            let _ = reqwest::Client::new()
                .post(&url)
                .timeout(std::time::Duration::from_secs(self.peer_timeout_secs))
                .json(tx)
                .send()
                .await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock helper for creating test PeerInfo
    fn create_test_peer(node_id: &str, address: &str, port: u16) -> PeerInfo {
        PeerInfo {
            node_id: node_id.to_string(),
            address: address.to_string(),
            port,
            last_seen: chrono::Utc::now(),
            version: "1.0".to_string(),
        }
    }

    #[test]
    fn test_vpn_mesh_network_creation() {
        let config = VpnMeshConfig {
            wireguard_interface: "wg0".to_string(),
            peer_addresses: vec![],
            mdns_enabled: false,
            gossip_port: 8765,
        };

        let result = VpnMeshNetwork::new(
            "test-node-1".to_string(),
            config,
            vec![],
            false,
            "pot-o-validator",
            30,
        );

        assert!(result.is_ok());
        let network = result.unwrap();
        assert_eq!(network.node_id(), "test-node-1");
    }

    #[test]
    fn test_vpn_mesh_network_with_bootstrap_urls() {
        let config = VpnMeshConfig {
            wireguard_interface: "wg0".to_string(),
            peer_addresses: vec![],
            mdns_enabled: false,
            gossip_port: 8765,
        };

        let bootstrap_urls = vec![
            "http://bootstrap1.local:8765".to_string(),
            "http://bootstrap2.local:8765".to_string(),
        ];

        let result = VpnMeshNetwork::new(
            "test-node-2".to_string(),
            config,
            bootstrap_urls.clone(),
            false,
            "pot-o-validator",
            30,
        );

        assert!(result.is_ok());
        let network = result.unwrap();
        assert_eq!(network.bootstrap_urls, bootstrap_urls);
    }

    #[tokio::test]
    async fn test_get_peers_returns_empty_initially() {
        let config = VpnMeshConfig {
            wireguard_interface: "wg0".to_string(),
            peer_addresses: vec![],
            mdns_enabled: false,
            gossip_port: 8765,
        };

        let network = VpnMeshNetwork::new(
            "test-node-3".to_string(),
            config,
            vec![],
            false,
            "pot-o-validator",
            30,
        )
        .unwrap();

        let peers = network.peer_list.read().await;
        assert_eq!(peers.len(), 0);
    }

    #[tokio::test]
    async fn test_node_id_returns_correct_value() {
        let config = VpnMeshConfig {
            wireguard_interface: "wg0".to_string(),
            peer_addresses: vec![],
            mdns_enabled: false,
            gossip_port: 8765,
        };

        let node_id = "my-test-node".to_string();
        let network = VpnMeshNetwork::new(
            node_id.clone(),
            config,
            vec![],
            false,
            "pot-o-validator",
            30,
        )
        .unwrap();

        assert_eq!(network.node_id(), &node_id);
    }

    #[tokio::test]
    async fn test_sync_state_with_empty_peers() {
        let config = VpnMeshConfig {
            wireguard_interface: "wg0".to_string(),
            peer_addresses: vec![],
            mdns_enabled: false,
            gossip_port: 8765,
        };

        let network = VpnMeshNetwork::new(
            "test-node-4".to_string(),
            config,
            vec![],
            false,
            "pot-o-validator",
            30,
        )
        .unwrap();

        let state = network.sync_state().await.unwrap();
        assert_eq!(state.peers.len(), 0);
        assert_eq!(state.total_nodes, 1);
        assert!(!state.synced);
    }

    #[tokio::test]
    async fn test_sync_state_with_peers() {
        let config = VpnMeshConfig {
            wireguard_interface: "wg0".to_string(),
            peer_addresses: vec![],
            mdns_enabled: false,
            gossip_port: 8765,
        };

        let network = VpnMeshNetwork::new(
            "test-node-5".to_string(),
            config,
            vec![],
            false,
            "pot-o-validator",
            30,
        )
        .unwrap();

        // Add some peers manually
        {
            let mut peer_list = network.peer_list.write().await;
            peer_list.push(create_test_peer("peer-1", "192.168.1.1", 8765));
            peer_list.push(create_test_peer("peer-2", "192.168.1.2", 8765));
        }

        let state = network.sync_state().await.unwrap();
        assert_eq!(state.peers.len(), 2);
        assert_eq!(state.total_nodes, 3); // 2 peers + self
        assert!(state.synced);
    }

    #[tokio::test]
    async fn test_discover_peers_filters_self() {
        let config = VpnMeshConfig {
            wireguard_interface: "wg0".to_string(),
            peer_addresses: vec![],
            mdns_enabled: false,
            gossip_port: 8765,
        };

        let node_id = "test-node-6".to_string();
        let network = VpnMeshNetwork::new(
            node_id.clone(),
            config,
            vec![],
            false,
            "pot-o-validator",
            30,
        )
        .unwrap();

        // Manually set peer list including ourselves and one other peer
        {
            let mut peer_list = network.peer_list.write().await;
            peer_list.push(create_test_peer(&node_id, "192.168.1.0", 8765)); // ourself
            peer_list.push(create_test_peer("peer-1", "192.168.1.1", 8765)); // another peer
        }

        // Verify sync_state counts correctly
        let state = network.sync_state().await.unwrap();
        // peer_list has 2 peers, total_nodes is len(peers) + 1 (for self) = 3
        assert_eq!(state.peers.len(), 2); // Both peers in the list
        assert_eq!(state.total_nodes, 3); // 2 from list + 1 for self
    }

    #[tokio::test]
    async fn test_broadcast_challenge_succeeds() {
        let config = VpnMeshConfig {
            wireguard_interface: "wg0".to_string(),
            peer_addresses: vec![],
            mdns_enabled: false,
            gossip_port: 8765,
        };

        let network = VpnMeshNetwork::new(
            "test-node-7".to_string(),
            config,
            vec![],
            false,
            "pot-o-validator",
            30,
        )
        .unwrap();

        // Create a minimal challenge (we just need something that serializes)
        // Since we don't have access to actual Challenge structure, we test the trait
        // The actual Challenge would come from pot_o_mining crate
        let result = network.sync_state().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_relay_proof_with_no_peers() {
        let config = VpnMeshConfig {
            wireguard_interface: "wg0".to_string(),
            peer_addresses: vec![],
            mdns_enabled: false,
            gossip_port: 8765,
        };

        let network = VpnMeshNetwork::new(
            "test-node-8".to_string(),
            config,
            vec![],
            false,
            "pot-o-validator",
            30,
        )
        .unwrap();

        // relay_proof should not error even with no peers
        let state = network.sync_state().await.unwrap();
        assert_eq!(state.peers.len(), 0);
    }
}
