//! Peer network: local-only and optional VPN mesh for multi-node discovery.

use async_trait::async_trait;
use pot_o_core::{TribeError, TribeResult};
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

// ---------------------------------------------------------------------------
// HexchainNetwork — P2P over raw TCP with geometric (HCP) peering
// ---------------------------------------------------------------------------

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use ed25519_dalek::{Keypair, Signature, Signer, Verifier};
use hexchain_p2p::lattice_geometry::{get_neighbors, HCPCoord};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

// ---------------------------------------------------------------------------
// Wire protocol (length-delimited borsh frames over TCP)
// ---------------------------------------------------------------------------

#[derive(Clone, BorshSerialize, BorshDeserialize)]
enum WireMessage {
    Handshake {
        node_id: String,
        coord_q: i32,
        coord_r: i32,
        coord_s: i32,
        pubkey: [u8; 32],
        signature: [u8; 64],
    },
    HandshakeAck {
        node_id: String,
        coord_q: i32,
        coord_r: i32,
        coord_s: i32,
    },
    ChallengeData {
        payload: Vec<u8>,
        ttl: u8,
        sender_q: i32,
        sender_r: i32,
        sender_s: i32,
    },
    ProofData {
        payload: Vec<u8>,
        ttl: u8,
        sender_q: i32,
        sender_r: i32,
        sender_s: i32,
    },
}

// ---------------------------------------------------------------------------
// Peer connection handle
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct PeerConnection {
    sender: mpsc::Sender<WireMessage>,
    peer_info: PeerInfo,
    coord: HCPCoord,
}

// ---------------------------------------------------------------------------
// HexchainNetwork
// ---------------------------------------------------------------------------

/// P2P network layer using geometric HCP peering over raw TCP + borsh + ed25519.
///
/// Each node derives its lattice coordinate from HASH(pubkey), computes its 12
/// geometric neighbors via `get_neighbors`, and maintains persistent TCP
/// connections to them. Messages are flooded with TTL=32.
pub struct HexchainNetwork {
    node_id: NodeId,
    coord: HCPCoord,
    #[allow(dead_code)]
    keypair: Arc<Keypair>,
    peers: Arc<RwLock<HashMap<String, PeerConnection>>>,
    #[allow(dead_code)]
    port: u16,
}

impl HexchainNetwork {
    /// Create a new HexchainNetwork, spawn a TCP listener and bootstrap connections.
    ///
    /// `bootstrap_nodes` format: `pubkey_hex@host:port` per entry, comma-separated.
    pub fn new(node_id: String, keypair: Keypair, port: u16, bootstrap_nodes: Vec<String>) -> Self {
        let pubkey_bytes = keypair.public.to_bytes();
        let coord = coord_from_pubkey(&pubkey_bytes);
        let keypair = Arc::new(keypair);
        let peers: Arc<RwLock<HashMap<String, PeerConnection>>> =
            Arc::new(RwLock::new(HashMap::new()));

        if port > 0 {
            let peers_clone = Arc::clone(&peers);
            let node_id_clone = node_id.clone();
            let keypair_clone = Arc::clone(&keypair);
            let my_coord = coord;
            tokio::spawn(async move {
                listener_loop(port, peers_clone, node_id_clone, keypair_clone, my_coord).await;
            });
        }

        if !bootstrap_nodes.is_empty() {
            let peers_clone = Arc::clone(&peers);
            let node_id_clone = node_id.clone();
            let keypair_clone = Arc::clone(&keypair);
            let my_coord = coord;
            tokio::spawn(async move {
                connect_bootstrap(
                    bootstrap_nodes,
                    peers_clone,
                    node_id_clone,
                    keypair_clone,
                    my_coord,
                )
                .await;
            });
        }

        Self {
            node_id,
            coord,
            keypair,
            peers,
            port,
        }
    }

    /// Compute this node's 12 geometric neighbors.
    pub fn neighbor_coords(&self) -> Vec<HCPCoord> {
        get_neighbors(self.coord).to_vec()
    }

    /// Borsh-serialize and flood a message to all connected peers.
    async fn flood(&self, payload: Vec<u8>, ttl: u8, is_challenge: bool) {
        let peers = self.peers.read().await;
        if peers.is_empty() {
            debug!("No connected peers to flood");
            return;
        }
        let msg = if is_challenge {
            WireMessage::ChallengeData {
                payload,
                ttl,
                sender_q: self.coord.q,
                sender_r: self.coord.r,
                sender_s: self.coord.s,
            }
        } else {
            WireMessage::ProofData {
                payload,
                ttl,
                sender_q: self.coord.q,
                sender_r: self.coord.r,
                sender_s: self.coord.s,
            }
        };
        for (id, conn) in peers.iter() {
            if conn.sender.try_send(msg.clone()).is_err() {
                debug!("Failed to send to peer {}", id);
            }
        }
    }
}

#[async_trait]
impl PeerNetwork for HexchainNetwork {
    fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    async fn discover_peers(&self) -> TribeResult<Vec<PeerInfo>> {
        let peers = self.peers.read().await;
        Ok(peers.values().map(|p| p.peer_info.clone()).collect())
    }

    async fn broadcast_challenge(&self, challenge: &Challenge) -> TribeResult<()> {
        let payload = serde_json::to_vec(challenge)
            .map_err(|e| TribeError::SerializationError(e.to_string()))?;
        self.flood(payload, 32, true).await;
        Ok(())
    }

    async fn relay_proof(&self, proof: &ProofPayload) -> TribeResult<()> {
        let payload =
            serde_json::to_vec(proof).map_err(|e| TribeError::SerializationError(e.to_string()))?;
        self.flood(payload, 32, false).await;
        Ok(())
    }

    async fn sync_state(&self) -> TribeResult<NetworkState> {
        let peers = self.peers.read().await;
        let peer_list: Vec<PeerInfo> = peers.values().map(|p| p.peer_info.clone()).collect();
        Ok(NetworkState {
            total_nodes: peer_list.len() + 1,
            synced: true,
            peers: peer_list,
        })
    }
}

// ---------------------------------------------------------------------------
// TCP listener — accepts incoming neighbor connections
// ---------------------------------------------------------------------------

async fn listener_loop(
    port: u16,
    peers: Arc<RwLock<HashMap<String, PeerConnection>>>,
    node_id: String,
    keypair: Arc<Keypair>,
    my_coord: HCPCoord,
) {
    let addr = format!("0.0.0.0:{}", port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => {
            info!("Hexchain P2P listening on {}", addr);
            l
        }
        Err(e) => {
            error!("Failed to bind P2P port {}: {}", port, e);
            return;
        }
    };

    loop {
        match listener.accept().await {
            Ok((stream, remote_addr)) => {
                let peers = Arc::clone(&peers);
                let node_id = node_id.clone();
                let keypair = Arc::clone(&keypair);
                tokio::spawn(async move {
                    handle_incoming(stream, remote_addr, peers, node_id, keypair, my_coord).await;
                });
            }
            Err(e) => {
                warn!("P2P accept error: {}", e);
            }
        }
    }
}

/// Handle an incoming TCP connection: handshake → store → read loop.
async fn handle_incoming(
    stream: TcpStream,
    remote_addr: SocketAddr,
    peers: Arc<RwLock<HashMap<String, PeerConnection>>>,
    node_id: String,
    _keypair: Arc<Keypair>,
    my_coord: HCPCoord,
) {
    let (mut reader, mut writer) = tokio::io::split(stream);

    // Read handshake
    let handshake = match recv_message_from_reader(&mut reader).await {
        Ok(WireMessage::Handshake {
            node_id: peer_id,
            coord_q,
            coord_r,
            coord_s,
            pubkey,
            signature,
        }) => (
            peer_id,
            HCPCoord {
                q: coord_q,
                r: coord_r,
                s: coord_s,
            },
            pubkey,
            signature,
        ),
        Ok(_) => {
            warn!("Expected handshake from {}", remote_addr);
            return;
        }
        Err(e) => {
            warn!("Handshake read error from {}: {}", remote_addr, e);
            return;
        }
    };

    let (peer_id, peer_coord, peer_pubkey, peer_sig) = handshake;

    // Verify handshake signature
    let mut sighash = Sha256::new();
    sighash.update(peer_id.as_bytes());
    sighash.update(peer_coord.q.to_le_bytes());
    sighash.update(peer_coord.r.to_le_bytes());
    sighash.update(peer_coord.s.to_le_bytes());
    let sighash = sighash.finalize();

    let verify_ok = ed25519_dalek::PublicKey::from_bytes(&peer_pubkey)
        .ok()
        .and_then(|pk| {
            Signature::from_bytes(&peer_sig)
                .ok()
                .map(|sig| pk.verify(&sighash, &sig).is_ok())
        })
        .unwrap_or(false);

    if !verify_ok {
        warn!(
            "Handshake signature verification failed from {}",
            remote_addr
        );
        return;
    }

    // Send acknowledgement
    let ack = WireMessage::HandshakeAck {
        node_id: node_id.clone(),
        coord_q: my_coord.q,
        coord_r: my_coord.r,
        coord_s: my_coord.s,
    };
    if send_message_to_writer(&mut writer, &ack).await.is_err() {
        warn!("Failed to send handshake ack to {}", remote_addr);
        return;
    }

    // Create channel for outgoing messages
    let (tx, mut rx) = mpsc::channel::<WireMessage>(64);

    let peer_info = PeerInfo {
        node_id: peer_id.clone(),
        address: format!("{}@{}", hex::encode(peer_pubkey), remote_addr),
        port: remote_addr.port(),
        last_seen: chrono::Utc::now(),
        version: "hexchain-0.1".into(),
    };

    {
        let mut peers_lock = peers.write().await;
        peers_lock.insert(
            peer_id.clone(),
            PeerConnection {
                sender: tx,
                peer_info: peer_info.clone(),
                coord: peer_coord,
            },
        );
    }

    info!(
        "P2P peer connected: {} at {} coord={:?}",
        peer_id, remote_addr, peer_coord
    );

    // Writer task: drain channel and send to stream
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if send_message_to_writer(&mut writer, &msg).await.is_err() {
                break;
            }
        }
    });

    // Read loop on the reader half
    loop {
        match recv_message_from_reader(&mut reader).await {
            Ok(WireMessage::ChallengeData { payload, ttl, .. }) => {
                if ttl > 1 {
                    let peers = peers.read().await;
                    let relay = WireMessage::ChallengeData {
                        payload: payload.clone(),
                        ttl: ttl - 1,
                        sender_q: my_coord.q,
                        sender_r: my_coord.r,
                        sender_s: my_coord.s,
                    };
                    for (id, conn) in peers.iter() {
                        if *id != peer_id {
                            let _ = conn.sender.try_send(relay.clone());
                        }
                    }
                }
            }
            Ok(WireMessage::ProofData { payload, ttl, .. }) => {
                if ttl > 1 {
                    let peers = peers.read().await;
                    let relay = WireMessage::ProofData {
                        payload: payload.clone(),
                        ttl: ttl - 1,
                        sender_q: my_coord.q,
                        sender_r: my_coord.r,
                        sender_s: my_coord.s,
                    };
                    for (id, conn) in peers.iter() {
                        if *id != peer_id {
                            let _ = conn.sender.try_send(relay.clone());
                        }
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                debug!("P2P read error from {}: {}", remote_addr, e);
                break;
            }
        }
    }

    // Cleanup on disconnect
    {
        let mut peers_lock = peers.write().await;
        peers_lock.remove(&peer_id);
    }
    info!("P2P peer disconnected: {}", peer_id);
}

// ---------------------------------------------------------------------------
// Bootstrap / neighbor connection
// ---------------------------------------------------------------------------

async fn connect_bootstrap(
    bootstrap_nodes: Vec<String>,
    peers: Arc<RwLock<HashMap<String, PeerConnection>>>,
    node_id: String,
    keypair: Arc<Keypair>,
    my_coord: HCPCoord,
) {
    for entry in &bootstrap_nodes {
        // Format: pubkey_hex@host:port
        let parts: Vec<&str> = entry.splitn(2, '@').collect();
        if parts.len() != 2 {
            warn!(
                "Invalid bootstrap node format (expected pubkey@host:port): {}",
                entry
            );
            continue;
        }
        let remote_addr = match parts[1].parse::<SocketAddr>() {
            Ok(a) => a,
            Err(e) => {
                warn!("Invalid bootstrap address {}: {}", parts[1], e);
                continue;
            }
        };

        debug!("Connecting to bootstrap node at {}", remote_addr);
        match TcpStream::connect(remote_addr).await {
            Ok(stream) => {
                handle_outgoing(
                    stream,
                    remote_addr,
                    peers.clone(),
                    node_id.clone(),
                    keypair.clone(),
                    my_coord,
                )
                .await;
            }
            Err(e) => {
                warn!("Failed to connect to bootstrap {}: {}", remote_addr, e);
            }
        }
    }
}

/// Complete outgoing connection: send handshake → receive ack → store → read loop.
async fn handle_outgoing(
    stream: TcpStream,
    remote_addr: SocketAddr,
    peers: Arc<RwLock<HashMap<String, PeerConnection>>>,
    node_id: String,
    keypair: Arc<Keypair>,
    my_coord: HCPCoord,
) {
    let (mut reader, mut writer) = tokio::io::split(stream);

    // Build signed handshake
    let mut sighash = Sha256::new();
    sighash.update(node_id.as_bytes());
    sighash.update(my_coord.q.to_le_bytes());
    sighash.update(my_coord.r.to_le_bytes());
    sighash.update(my_coord.s.to_le_bytes());
    let sighash = sighash.finalize();
    let signature = keypair.sign(&sighash);

    let handshake = WireMessage::Handshake {
        node_id: node_id.clone(),
        coord_q: my_coord.q,
        coord_r: my_coord.r,
        coord_s: my_coord.s,
        pubkey: keypair.public.to_bytes(),
        signature: signature.to_bytes(),
    };

    if send_message_to_writer(&mut writer, &handshake)
        .await
        .is_err()
    {
        warn!("Failed to send handshake to {}", remote_addr);
        return;
    }

    // Read acknowledgement
    let (peer_id, peer_coord) = match recv_message_from_reader(&mut reader).await {
        Ok(WireMessage::HandshakeAck {
            node_id: pid,
            coord_q,
            coord_r,
            coord_s,
        }) => (
            pid,
            HCPCoord {
                q: coord_q,
                r: coord_r,
                s: coord_s,
            },
        ),
        Ok(_) => {
            warn!("Expected HandshakeAck from {}", remote_addr);
            return;
        }
        Err(e) => {
            warn!("Handshake ack read error from {}: {}", remote_addr, e);
            return;
        }
    };

    let (tx, mut rx) = mpsc::channel::<WireMessage>(64);

    let peer_info = PeerInfo {
        node_id: peer_id.clone(),
        address: remote_addr.to_string(),
        port: remote_addr.port(),
        last_seen: chrono::Utc::now(),
        version: "hexchain-0.1".into(),
    };

    {
        let mut peers_lock = peers.write().await;
        peers_lock.insert(
            peer_id.clone(),
            PeerConnection {
                sender: tx,
                peer_info: peer_info.clone(),
                coord: peer_coord,
            },
        );
    }

    info!(
        "P2P connected to peer: {} at {} coord={:?}",
        peer_id, remote_addr, peer_coord
    );

    // Writer task
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if send_message_to_writer(&mut writer, &msg).await.is_err() {
                break;
            }
        }
    });

    // Reader loop on reader half
    loop {
        match recv_message_from_reader(&mut reader).await {
            Ok(WireMessage::ChallengeData { payload, ttl, .. }) => {
                if ttl > 1 {
                    let peers = peers.read().await;
                    let relay = WireMessage::ChallengeData {
                        payload: payload.clone(),
                        ttl: ttl - 1,
                        sender_q: my_coord.q,
                        sender_r: my_coord.r,
                        sender_s: my_coord.s,
                    };
                    for (id, conn) in peers.iter() {
                        if *id != peer_id {
                            let _ = conn.sender.try_send(relay.clone());
                        }
                    }
                }
            }
            Ok(WireMessage::ProofData { payload, ttl, .. }) => {
                if ttl > 1 {
                    let peers = peers.read().await;
                    let relay = WireMessage::ProofData {
                        payload: payload.clone(),
                        ttl: ttl - 1,
                        sender_q: my_coord.q,
                        sender_r: my_coord.r,
                        sender_s: my_coord.s,
                    };
                    for (id, conn) in peers.iter() {
                        if *id != peer_id {
                            let _ = conn.sender.try_send(relay.clone());
                        }
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                debug!("P2P read error from {}: {}", remote_addr, e);
                break;
            }
        }
    }

    {
        let mut peers_lock = peers.write().await;
        peers_lock.remove(&peer_id);
    }
    info!("P2P peer disconnected: {}", peer_id);
}

// ---------------------------------------------------------------------------
// Wire I/O helpers (operate on split reader/writer halves)
// ---------------------------------------------------------------------------

use tokio::io::ReadHalf;

type TcpReader = ReadHalf<TcpStream>;
type TcpWriter = tokio::io::WriteHalf<TcpStream>;

async fn send_message_to_writer(
    writer: &mut TcpWriter,
    msg: &WireMessage,
) -> Result<(), TribeError> {
    let bytes = borsh::to_vec(msg).map_err(|e| TribeError::SerializationError(e.to_string()))?;
    let len = (bytes.len() as u32).to_le_bytes();
    writer.write_all(&len).await.map_err(TribeError::IoError)?;
    writer
        .write_all(&bytes)
        .await
        .map_err(TribeError::IoError)?;
    Ok(())
}

async fn recv_message_from_reader(reader: &mut TcpReader) -> Result<WireMessage, TribeError> {
    let mut len_buf = [0u8; 4];
    reader
        .read_exact(&mut len_buf)
        .await
        .map_err(TribeError::IoError)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    reader
        .read_exact(&mut buf)
        .await
        .map_err(TribeError::IoError)?;
    WireMessage::try_from_slice(&buf).map_err(|e| TribeError::SerializationError(e.to_string()))
}

// ---------------------------------------------------------------------------
// Utility: derive HCP coordinate from ed25519 pubkey
// ---------------------------------------------------------------------------

/// Maps a 32-byte ed25519 public key to an HCP lattice coordinate (q + r + s = 0).
///
/// Uses SHA256(pubkey) → (q, r) from first 8 bytes, s = -(q + r).
/// Result is modded to [-1023, 1023] range so nodes start near the origin cluster.
fn coord_from_pubkey(pubkey: &[u8; 32]) -> HCPCoord {
    let hash = Sha256::digest(pubkey);
    let q = (i32::from_le_bytes(hash[0..4].try_into().unwrap()) % 1024).max(-1023);
    let r = (i32::from_le_bytes(hash[4..8].try_into().unwrap()) % 1024).max(-1023);
    let s = -(q + r);
    HCPCoord { q, r, s }
}
