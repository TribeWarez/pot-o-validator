//! Peer network: local-only and optional VPN mesh for multi-node discovery.

use async_trait::async_trait;
use pot_o_core::TribeResult;
use pot_o_mining::{Challenge, ProofPayload};
use serde::{Deserialize, Serialize};

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
// VpnMeshNetwork (stubbed for future WireGuard + mDNS)
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
}

#[async_trait]
impl PeerNetwork for VpnMeshNetwork {
    fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    async fn discover_peers(&self) -> TribeResult<Vec<PeerInfo>> {
        todo!("VPN mesh peer discovery via WireGuard + mDNS not yet implemented")
    }

    async fn broadcast_challenge(&self, _challenge: &Challenge) -> TribeResult<()> {
        todo!("VPN mesh challenge broadcast not yet implemented")
    }

    async fn relay_proof(&self, _proof: &ProofPayload) -> TribeResult<()> {
        todo!("VPN mesh proof relay not yet implemented")
    }

    async fn sync_state(&self) -> TribeResult<NetworkState> {
        todo!("VPN mesh state sync not yet implemented")
    }
}
