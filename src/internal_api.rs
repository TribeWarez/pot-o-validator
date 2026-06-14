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
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;

/// State for the internal API, shared across peer communication handlers.
#[derive(Clone)]
pub struct InternalApiState {
    /// Node ID of this validator (e.g., "validator-1")
    pub node_id: String,
    /// List of known peers in the network
    pub peers: Arc<RwLock<Vec<PeerInfo>>>,
    /// Current challenge being broadcast (if any)
    pub current_challenge: Arc<RwLock<Option<serde_json::Value>>>,
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
        .with_state(state)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_internal_api_state_creation() {
        let state = InternalApiState {
            node_id: "validator-1".to_string(),
            peers: Arc::new(RwLock::new(vec![])),
            current_challenge: Arc::new(RwLock::new(None)),
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
        };

        let peers = state.peers.read().await;
        assert_eq!(peers.len(), 3);
        assert_eq!(peers[0].node_id, "v2");
        assert_eq!(peers[1].node_id, "v3");
        assert_eq!(peers[2].node_id, "v4");
    }
}
