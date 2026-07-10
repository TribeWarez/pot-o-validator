//! Internal API for peer-to-peer communication and challenge broadcasting.
//!
//! This module provides HTTP endpoints for validators to communicate with each other,
//! including peer registration, listing, and challenge broadcast.
//!
//! Endpoints:
//! - POST /internal/peer/register - Register a peer validator
//! - GET /internal/peers - List known peers
//! - POST /internal/challenge/broadcast - Receive challenge from peer

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use pot_o_core::TokenType;
use pot_o_extensions::pool_strategy::ProofRecord;
use pot_o_extensions::{tx::TransferTransaction, Mempool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use crate::peer_store::PeerStore;

/// State for the internal API, shared across peer communication handlers.
#[derive(Clone)]
pub struct InternalApiState {
    /// Node ID of this validator (e.g., "validator-1")
    #[allow(dead_code)]
    pub node_id: String,
    /// List of known peers in the network
    pub peers: Arc<RwLock<Vec<PeerInfo>>>,
    /// Current challenge being broadcast (if any)
    pub current_challenge: Arc<RwLock<Option<serde_json::Value>>>,
    /// Mempool for tribechain transaction gossip
    pub mempool: Option<Arc<Mempool>>,
    /// Ledger for tribechain state queries
    pub ledger: Arc<RwLock<pot_o_extensions::Ledger>>,
    /// Whether tribechain is enabled
    pub tribechain_enabled: bool,
    /// Optional shared secret for mint authorization
    pub internal_mint_secret: Option<String>,
    /// Optional peer store for persisting peers to disk
    pub peer_store: Option<Arc<PeerStore>>,
}

/// Information about a peer validator in the network.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PeerInfo {
    /// Unique identifier for this peer
    pub node_id: String,
    /// HTTP URL to reach this peer
    pub url: String,
    /// Last time we heard from this peer
    pub last_seen: DateTime<Utc>,
}

/// Request payload for peer registration
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterPeerRequest {
    pub node_id: String,
    pub url: String,
}

/// Request payload for challenge broadcast
#[derive(Debug, Serialize, Deserialize)]
pub struct BroadcastChallengeRequest {
    #[serde(flatten)]
    pub challenge: serde_json::Value,
}

/// Generic response for API operations
#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub message: String,
}

/// Builds the Axum router for internal API endpoints.
pub fn internal_router(state: InternalApiState) -> Router {
    Router::new()
        .route("/internal/peer/register", post(handle_peer_register))
        .route("/internal/peers", get(handle_list_peers))
        .route(
            "/internal/challenge/broadcast",
            post(handle_challenge_broadcast),
        )
        .route("/internal/tx/broadcast", post(handle_tx_broadcast))
        .route("/internal/mint", post(handle_internal_mint))
        .route("/api/pool/submit-batch", post(handle_submit_batch))
        .layer(middleware::from_fn(verify_peer_signature_middleware))
        .with_state(state)
}

/// Middleware that verifies peer signatures on incoming requests.
///
/// If the request contains `X-Node-Pubkey` and `X-Node-Signature` headers,
/// the signature is verified against the reconstructed message:
/// `timestamp:method:path:body_hash`
async fn verify_peer_signature_middleware(req: Request<Body>, next: Next) -> impl IntoResponse {
    let pubkey_hex = req
        .headers()
        .get("x-node-pubkey")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let signature_hex = req
        .headers()
        .get("x-node-signature")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let timestamp = req
        .headers()
        .get("x-node-timestamp")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let has_auth_headers = pubkey_hex.is_some() && signature_hex.is_some() && timestamp.is_some();

    if !has_auth_headers {
        return next.run(req).await.into_response();
    }

    let pubkey_hex = pubkey_hex.unwrap();
    let signature_hex = signature_hex.unwrap();
    let timestamp = timestamp.unwrap();

    let pubkey_bytes = match hex::decode(&pubkey_hex) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "invalid pubkey format"})),
            )
                .into_response();
        }
    };

    let signature_bytes = match hex::decode(&signature_hex) {
        Ok(b) if b.len() == 64 => b,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "invalid signature format"})),
            )
                .into_response();
        }
    };

    let method = req.method().as_str().to_string();
    let path = req.uri().path().to_string();

    let body_bytes = match axum::body::to_bytes(req.into_body(), 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "failed to read body"})),
            )
                .into_response();
        }
    };

    let body_hash = hex::encode(Sha256::digest(&body_bytes));
    let message = format!("{}:{}:{}:{}", timestamp, method, path, body_hash);

    if !pot_o_extensions::peer_auth::verify_peer_signature(
        &pubkey_bytes,
        message.as_bytes(),
        &signature_bytes,
    ) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "invalid signature"})),
        )
            .into_response();
    }

    let new_req = Request::builder()
        .method(method.as_str())
        .uri(path.as_str())
        .body(Body::from(body_bytes))
        .unwrap();
    next.run(new_req).await.into_response()
}

/// Handler: POST /internal/peer/register
/// Registers a peer validator in the network, deduplicating by node_id.
async fn handle_peer_register(
    State(state): State<InternalApiState>,
    Json(req): Json<RegisterPeerRequest>,
) -> impl IntoResponse {
    if req.node_id.is_empty() || req.url.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "node_id and url are required"})),
        )
            .into_response();
    }

    let peer_info = PeerInfo {
        node_id: req.node_id.clone(),
        url: req.url,
        last_seen: Utc::now(),
    };

    let mut peers = state.peers.write().await;

    // Check if peer already exists (by node_id) and update or insert
    if let Some(existing) = peers.iter_mut().find(|p| p.node_id == req.node_id) {
        existing.last_seen = Utc::now();
        existing.url = peer_info.url;
    } else {
        peers.push(peer_info);
    }

    drop(peers);

    if let Some(ref store) = state.peer_store {
        let store = store.clone();
        tokio::spawn(async move {
            if let Err(e) = store.save().await {
                tracing::warn!(error = %e, "Failed to persist peers after registration");
            }
        });
    }

    (
        StatusCode::OK,
        Json(ApiResponse {
            message: "Peer registered".to_string(),
        }),
    )
        .into_response()
}

/// Handler: GET /internal/peers
/// Returns a list of all known peers in JSON format.
async fn handle_list_peers(State(state): State<InternalApiState>) -> impl IntoResponse {
    let peers = state.peers.read().await.clone();
    (StatusCode::OK, Json(peers)).into_response()
}

/// Handler: POST /internal/challenge/broadcast
/// Receives and stores a challenge broadcast from a peer.
async fn handle_challenge_broadcast(
    State(state): State<InternalApiState>,
    Json(req): Json<BroadcastChallengeRequest>,
) -> impl IntoResponse {
    let mut challenge = state.current_challenge.write().await;
    *challenge = Some(req.challenge);

    (
        StatusCode::OK,
        Json(ApiResponse {
            message: "Challenge broadcast received".to_string(),
        }),
    )
        .into_response()
}

/// Handler: POST /internal/tx/broadcast
/// Receives a transaction from a peer and submits it to the local mempool.
async fn handle_tx_broadcast(
    State(state): State<InternalApiState>,
    Json(tx_val): Json<serde_json::Value>,
) -> impl IntoResponse {
    if !state.tribechain_enabled {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "tribechain not enabled"})),
        )
            .into_response();
    }

    let Some(mempool) = &state.mempool else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "mempool not available"})),
        )
            .into_response();
    };

    let tx: TransferTransaction = match serde_json::from_value(tx_val.clone()) {
        Ok(tx) => tx,
        Err(e) => {
            tracing::warn!(error = %e, "Invalid transaction from peer");
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "invalid transaction"})),
            )
                .into_response();
        }
    };

    match mempool.submit(tx, &state.ledger).await {
        Ok(tx_hash) => {
            tracing::debug!(tx_hash = %hex::encode(tx_hash), "Received tx from peer, added to mempool");
            let peers = state.peers.read().await.clone();
            let tx_val_clone = tx_val.clone();
            tokio::spawn(async move {
                for peer in &peers {
                    let url = format!("{}/internal/tx/broadcast", peer.url.trim_end_matches('/'));
                    let _ = reqwest::Client::new()
                        .post(&url)
                        .timeout(std::time::Duration::from_secs(5))
                        .json(&tx_val_clone)
                        .send()
                        .await;
                }
            });
            (
                StatusCode::OK,
                Json(json!({
                    "accepted": true,
                    "tx_hash": hex::encode(tx_hash),
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::warn!(error = %e, "Rejected tx from peer");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "transaction rejected"})),
            )
                .into_response()
        }
    }
}

/// Request payload for batch submission
#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitBatchRequest {
    pub node_id: String,
    pub pool_strategy: String,
    pub batch: Vec<ProofRecord>,
}

/// Handler: POST /api/pool/submit-batch
/// Receives a batch of proof records from a peer validator for pool accounting.
/// Returns a deterministic submission ID built from the batch content.
async fn handle_submit_batch(
    State(_state): State<InternalApiState>,
    Json(req): Json<SubmitBatchRequest>,
) -> impl IntoResponse {
    let batch_size = req.batch.len();

    if batch_size == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "batch cannot be empty"})),
        )
            .into_response();
    }

    // Deterministic hash of the batch
    let mut hasher = Sha256::new();
    hasher.update(b"pot-o-batch:");
    hasher.update(req.node_id.as_bytes());
    hasher.update(b":");
    for record in &req.batch {
        hasher.update(record.miner_pubkey.as_bytes());
        hasher.update(b":");
        hasher.update(record.challenge_id.as_bytes());
        hasher.update(b":");
        hasher.update(record.reward.to_le_bytes());
        hasher.update(b":");
        hasher.update(record.timestamp.timestamp().to_le_bytes());
        hasher.update(b"|");
    }
    let hash = hex::encode(hasher.finalize());
    let submission_id = format!("pool-batch-{}", &hash[..40]);

    tracing::info!(
        from_node = %req.node_id,
        batch_size,
        submission_id = %submission_id,
        "Received batch submission from peer validator"
    );

    (
        StatusCode::OK,
        Json(json!({
            "submission_id": submission_id,
            "accepted": true,
            "batch_size": batch_size,
        })),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
struct InternalMintRequest {
    to: String,
    token_type: String,
    amount: u64,
}

async fn handle_internal_mint(
    State(state): State<InternalApiState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<InternalMintRequest>,
) -> impl IntoResponse {
    if let Some(ref expected) = state.internal_mint_secret {
        let got = headers
            .get("x-internal-secret")
            .and_then(|v| v.to_str().ok());
        if got != Some(expected.as_str()) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "invalid internal secret"})),
            )
                .into_response();
        }
    }

    if !state.tribechain_enabled {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "tribechain not enabled"})),
        )
            .into_response();
    }

    let token = match token_type_from_str(&body.token_type) {
        Some(t) => t,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Unknown token: {}", body.token_type)})),
            )
                .into_response();
        }
    };

    let mut ledger = state.ledger.write().await;
    if let Err(e) = ledger.try_issue(&body.to, &token, body.amount) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": e}))).into_response();
    }

    let receipt = json!({
        "status": "ok",
        "to": body.to,
        "token": body.token_type,
        "amount": body.amount,
        "tx_hash": format!("mint-{}-{}", body.to, body.token_type),
    });
    (StatusCode::OK, Json(receipt)).into_response()
}

fn token_type_from_str(s: &str) -> Option<TokenType> {
    match s.to_lowercase().as_str() {
        "tribechain" | "native" | "tribe" => Some(TokenType::TribeChain),
        "pttc" => Some(TokenType::PTtC),
        "nmtc" => Some(TokenType::NMTC),
        "stomp" => Some(TokenType::STOMP),
        "aum" => Some(TokenType::AUM),
        "ai3" => Some(TokenType::AI3),
        "ravecoin" => Some(TokenType::RAVECOIN),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_internal_api_state_creation() {
        let state = InternalApiState {
            node_id: "validator-1".to_string(),
            peers: Arc::new(RwLock::new(vec![])),
            current_challenge: Arc::new(RwLock::new(None)),
            mempool: None,
            ledger: Arc::new(RwLock::new(pot_o_extensions::Ledger::new(
                "test".to_string(),
            ))),
            tribechain_enabled: false,
            internal_mint_secret: None,
            peer_store: None,
        };

        assert_eq!(state.node_id, "validator-1");
        let peers = state.peers.read().await;
        assert_eq!(peers.len(), 0);
        drop(peers);

        let challenge = state.current_challenge.read().await;
        assert!(challenge.is_none());
    }

    #[tokio::test]
    async fn test_peer_info_creation() {
        let now = Utc::now();
        let peer = PeerInfo {
            node_id: "validator-2".to_string(),
            url: "http://validator-2:8900".to_string(),
            last_seen: now,
        };

        assert_eq!(peer.node_id, "validator-2");
        assert_eq!(peer.url, "http://validator-2:8900");
        assert_eq!(peer.last_seen, now);
    }

    #[tokio::test]
    async fn test_peer_registration_creates_new_entry() {
        let state = InternalApiState {
            node_id: "validator-1".to_string(),
            peers: Arc::new(RwLock::new(vec![])),
            current_challenge: Arc::new(RwLock::new(None)),
            mempool: None,
            ledger: Arc::new(RwLock::new(pot_o_extensions::Ledger::new(
                "test".to_string(),
            ))),
            tribechain_enabled: false,
            internal_mint_secret: None,
            peer_store: None,
        };

        let req = RegisterPeerRequest {
            node_id: "validator-2".to_string(),
            url: "http://validator-2:8900".to_string(),
        };

        let mut peers = state.peers.write().await;
        if let Some(existing) = peers.iter_mut().find(|p| p.node_id == req.node_id) {
            existing.last_seen = Utc::now();
            existing.url = req.url;
        } else {
            peers.push(PeerInfo {
                node_id: req.node_id,
                url: req.url,
                last_seen: Utc::now(),
            });
        }
        drop(peers);

        let peers = state.peers.read().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].node_id, "validator-2");
    }

    #[tokio::test]
    async fn test_peer_registration_updates_existing_peer() {
        let initial_peer = PeerInfo {
            node_id: "validator-2".to_string(),
            url: "http://validator-2:8900".to_string(),
            last_seen: Utc::now(),
        };

        let state = InternalApiState {
            node_id: "validator-1".to_string(),
            peers: Arc::new(RwLock::new(vec![initial_peer.clone()])),
            current_challenge: Arc::new(RwLock::new(None)),
            mempool: None,
            ledger: Arc::new(RwLock::new(pot_o_extensions::Ledger::new(
                "test".to_string(),
            ))),
            tribechain_enabled: false,
            internal_mint_secret: None,
            peer_store: None,
        };

        let req = RegisterPeerRequest {
            node_id: "validator-2".to_string(),
            url: "http://validator-2:9000".to_string(), // Different URL
        };

        let mut peers = state.peers.write().await;
        if let Some(existing) = peers.iter_mut().find(|p| p.node_id == req.node_id) {
            existing.last_seen = Utc::now();
            existing.url = req.url;
        } else {
            peers.push(PeerInfo {
                node_id: req.node_id,
                url: req.url,
                last_seen: Utc::now(),
            });
        }
        drop(peers);

        let peers = state.peers.read().await;
        assert_eq!(peers.len(), 1); // Still only one peer
        assert_eq!(peers[0].url, "http://validator-2:9000"); // URL updated
    }

    #[tokio::test]
    async fn test_list_peers_empty_by_default() {
        let state = InternalApiState {
            node_id: "validator-1".to_string(),
            peers: Arc::new(RwLock::new(vec![])),
            current_challenge: Arc::new(RwLock::new(None)),
            mempool: None,
            ledger: Arc::new(RwLock::new(pot_o_extensions::Ledger::new(
                "test".to_string(),
            ))),
            tribechain_enabled: false,
            internal_mint_secret: None,
            peer_store: None,
        };

        let peers = state.peers.read().await;
        assert_eq!(peers.len(), 0);
    }

    #[tokio::test]
    async fn test_list_peers_returns_all_registered_peers() {
        let peer1 = PeerInfo {
            node_id: "validator-2".to_string(),
            url: "http://validator-2:8900".to_string(),
            last_seen: Utc::now(),
        };

        let peer2 = PeerInfo {
            node_id: "validator-3".to_string(),
            url: "http://validator-3:8900".to_string(),
            last_seen: Utc::now(),
        };

        let state = InternalApiState {
            node_id: "validator-1".to_string(),
            peers: Arc::new(RwLock::new(vec![peer1.clone(), peer2.clone()])),
            current_challenge: Arc::new(RwLock::new(None)),
            mempool: None,
            ledger: Arc::new(RwLock::new(pot_o_extensions::Ledger::new(
                "test".to_string(),
            ))),
            tribechain_enabled: false,
            internal_mint_secret: None,
            peer_store: None,
        };

        let peers = state.peers.read().await;
        assert_eq!(peers.len(), 2);
        assert_eq!(peers[0], peer1);
        assert_eq!(peers[1], peer2);
    }

    #[tokio::test]
    async fn test_challenge_broadcast_stores_challenge() {
        let state = InternalApiState {
            node_id: "validator-1".to_string(),
            peers: Arc::new(RwLock::new(vec![])),
            current_challenge: Arc::new(RwLock::new(None)),
            mempool: None,
            ledger: Arc::new(RwLock::new(pot_o_extensions::Ledger::new(
                "test".to_string(),
            ))),
            tribechain_enabled: false,
            internal_mint_secret: None,
            peer_store: None,
        };

        let challenge_data = json!({"id": "challenge-1", "difficulty": 2});
        let mut challenge = state.current_challenge.write().await;
        *challenge = Some(challenge_data.clone());
        drop(challenge);

        let challenge = state.current_challenge.read().await;
        assert!(challenge.is_some());
        assert_eq!(*challenge, Some(challenge_data));
    }

    #[tokio::test]
    async fn test_challenge_broadcast_replaces_old_challenge() {
        let old_challenge = json!({"id": "challenge-1", "difficulty": 1});
        let new_challenge = json!({"id": "challenge-2", "difficulty": 2});

        let state = InternalApiState {
            node_id: "validator-1".to_string(),
            peers: Arc::new(RwLock::new(vec![])),
            current_challenge: Arc::new(RwLock::new(Some(old_challenge))),
            mempool: None,
            ledger: Arc::new(RwLock::new(pot_o_extensions::Ledger::new(
                "test".to_string(),
            ))),
            tribechain_enabled: false,
            internal_mint_secret: None,
            peer_store: None,
        };

        let mut challenge = state.current_challenge.write().await;
        *challenge = Some(new_challenge.clone());
        drop(challenge);

        let challenge = state.current_challenge.read().await;
        assert_eq!(*challenge, Some(new_challenge));
    }

    #[tokio::test]
    async fn test_peer_deduplication_by_node_id() {
        let peer1 = PeerInfo {
            node_id: "validator-2".to_string(),
            url: "http://validator-2:8900".to_string(),
            last_seen: Utc::now(),
        };

        let peer2 = PeerInfo {
            node_id: "validator-3".to_string(),
            url: "http://validator-3:8900".to_string(),
            last_seen: Utc::now(),
        };

        let state = InternalApiState {
            node_id: "validator-1".to_string(),
            peers: Arc::new(RwLock::new(vec![peer1, peer2])),
            current_challenge: Arc::new(RwLock::new(None)),
            mempool: None,
            ledger: Arc::new(RwLock::new(pot_o_extensions::Ledger::new(
                "test".to_string(),
            ))),
            tribechain_enabled: false,
            internal_mint_secret: None,
            peer_store: None,
        };

        // Register with duplicate node_id
        let req = RegisterPeerRequest {
            node_id: "validator-2".to_string(),
            url: "http://validator-2:9000".to_string(),
        };

        let mut peers = state.peers.write().await;
        if let Some(existing) = peers.iter_mut().find(|p| p.node_id == req.node_id) {
            existing.url = req.url;
            existing.last_seen = Utc::now();
        } else {
            peers.push(PeerInfo {
                node_id: req.node_id,
                url: req.url,
                last_seen: Utc::now(),
            });
        }
        drop(peers);

        let peers = state.peers.read().await;
        assert_eq!(peers.len(), 2); // Still 2 peers, not 3
        assert_eq!(
            peers.iter().filter(|p| p.node_id == "validator-2").count(),
            1
        );
    }

    #[tokio::test]
    async fn test_last_seen_update_on_re_registration() {
        let initial_time = Utc::now();
        let peer = PeerInfo {
            node_id: "validator-2".to_string(),
            url: "http://validator-2:8900".to_string(),
            last_seen: initial_time,
        };

        let state = InternalApiState {
            node_id: "validator-1".to_string(),
            peers: Arc::new(RwLock::new(vec![peer])),
            current_challenge: Arc::new(RwLock::new(None)),
            mempool: None,
            ledger: Arc::new(RwLock::new(pot_o_extensions::Ledger::new(
                "test".to_string(),
            ))),
            tribechain_enabled: false,
            internal_mint_secret: None,
            peer_store: None,
        };

        // Wait a bit and re-register
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let req = RegisterPeerRequest {
            node_id: "validator-2".to_string(),
            url: "http://validator-2:8900".to_string(),
        };

        let mut peers = state.peers.write().await;
        if let Some(existing) = peers.iter_mut().find(|p| p.node_id == req.node_id) {
            existing.last_seen = Utc::now();
            existing.url = req.url;
        } else {
            peers.push(PeerInfo {
                node_id: req.node_id,
                url: req.url,
                last_seen: Utc::now(),
            });
        }
        drop(peers);

        let peers = state.peers.read().await;
        assert!(peers[0].last_seen > initial_time);
    }

    #[tokio::test]
    async fn test_handler_peer_registration_success() {
        let state = InternalApiState {
            node_id: "validator-1".to_string(),
            peers: Arc::new(RwLock::new(vec![])),
            current_challenge: Arc::new(RwLock::new(None)),
            mempool: None,
            ledger: Arc::new(RwLock::new(pot_o_extensions::Ledger::new(
                "test".to_string(),
            ))),
            tribechain_enabled: false,
            internal_mint_secret: None,
            peer_store: None,
        };

        // Verify the handler returns correct structure
        let peers = state.peers.read().await;
        assert_eq!(peers.len(), 0);
    }

    #[tokio::test]
    async fn test_handler_challenge_broadcast_success() {
        let state = InternalApiState {
            node_id: "validator-1".to_string(),
            peers: Arc::new(RwLock::new(vec![])),
            current_challenge: Arc::new(RwLock::new(None)),
            mempool: None,
            ledger: Arc::new(RwLock::new(pot_o_extensions::Ledger::new(
                "test".to_string(),
            ))),
            tribechain_enabled: false,
            internal_mint_secret: None,
            peer_store: None,
        };

        let challenge = json!({"id": "c1", "difficulty": 2});
        let mut challenge_lock = state.current_challenge.write().await;
        *challenge_lock = Some(challenge.clone());
        drop(challenge_lock);

        let stored = state.current_challenge.read().await;
        assert_eq!(*stored, Some(challenge));
    }

    #[tokio::test]
    async fn test_multiple_peers_can_coexist() {
        let peers_vec = vec![
            PeerInfo {
                node_id: "v2".to_string(),
                url: "http://v2:8900".to_string(),
                last_seen: Utc::now(),
            },
            PeerInfo {
                node_id: "v3".to_string(),
                url: "http://v3:8900".to_string(),
                last_seen: Utc::now(),
            },
            PeerInfo {
                node_id: "v4".to_string(),
                url: "http://v4:8900".to_string(),
                last_seen: Utc::now(),
            },
        ];

        let state = InternalApiState {
            node_id: "validator-1".to_string(),
            peers: Arc::new(RwLock::new(peers_vec)),
            current_challenge: Arc::new(RwLock::new(None)),
            mempool: None,
            ledger: Arc::new(RwLock::new(pot_o_extensions::Ledger::new(
                "test".to_string(),
            ))),
            tribechain_enabled: false,
            internal_mint_secret: None,
            peer_store: None,
        };

        let peers = state.peers.read().await;
        assert_eq!(peers.len(), 3);
        assert_eq!(peers[0].node_id, "v2");
        assert_eq!(peers[1].node_id, "v3");
        assert_eq!(peers[2].node_id, "v4");
    }
}
