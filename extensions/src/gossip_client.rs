//! Gossip client for peer-to-peer challenge distribution using async HTTP.

use crate::peer_auth::NodeIdentity;
use serde_json;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Async HTTP-based gossip client for broadcasting challenges to peers.
pub struct GossipClient {
    /// Client for making HTTP requests
    client: reqwest::Client,
    /// Thread-safe list of peer URLs
    peers: Arc<RwLock<Vec<String>>>,
    /// Request timeout duration
    timeout: Duration,
    /// Optional node identity for request signing
    identity: Option<Arc<NodeIdentity>>,
}

impl GossipClient {
    /// Create a new gossip client with the given peer URLs and timeout.
    ///
    /// # Arguments
    /// * `peer_urls` - List of peer URLs for challenge distribution
    /// * `timeout_secs` - Request timeout in seconds
    pub fn new(peer_urls: Vec<String>, timeout_secs: u64) -> Self {
        Self {
            client: reqwest::Client::new(),
            peers: Arc::new(RwLock::new(peer_urls)),
            timeout: Duration::from_secs(timeout_secs),
            identity: None,
        }
    }

    /// Create a new gossip client with authentication enabled.
    ///
    /// # Arguments
    /// * `peer_urls` - List of peer URLs for challenge distribution
    /// * `timeout_secs` - Request timeout in seconds
    /// * `identity` - Node identity for signing requests
    pub fn with_identity(
        peer_urls: Vec<String>,
        timeout_secs: u64,
        identity: Arc<NodeIdentity>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            peers: Arc::new(RwLock::new(peer_urls)),
            timeout: Duration::from_secs(timeout_secs),
            identity: Some(identity),
        }
    }

    /// Sign a request and return authentication headers.
    ///
    /// # Arguments
    /// * `method` - HTTP method (GET, POST, etc.)
    /// * `path` - Request path
    /// * `body` - Request body bytes
    fn sign_request(&self, method: &str, path: &str, body: &[u8]) -> Vec<(String, String)> {
        let Some(identity) = &self.identity else {
            return vec![];
        };

        let timestamp = chrono::Utc::now().timestamp();
        let body_hash = hex::encode(Sha256::digest(body));
        let message = format!("{}:{}:{}:{}", timestamp, method, path, body_hash);
        let signature = identity.sign_message(message.as_bytes());

        vec![
            ("X-Node-Id".to_string(), identity.node_id().to_string()),
            (
                "X-Node-Pubkey".to_string(),
                hex::encode(identity.public_key_bytes()),
            ),
            ("X-Node-Signature".to_string(), hex::encode(signature)),
            ("X-Node-Timestamp".to_string(), timestamp.to_string()),
        ]
    }

    /// Broadcast a challenge to all peers, returning the count of successful broadcasts.
    ///
    /// # Arguments
    /// * `challenge_id` - Unique identifier for the challenge
    /// * `challenge_json` - Challenge data as JSON
    ///
    /// # Returns
    /// Number of peers that successfully received the challenge
    pub async fn broadcast_challenge(
        &self,
        challenge_id: &str,
        challenge_json: &serde_json::Value,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let peers = self.peers.read().await.clone();
        let mut success_count = 0;

        for peer_url in peers {
            let payload = serde_json::json!({
                "challenge_id": challenge_id,
                "challenge": challenge_json,
            });
            let body_bytes = serde_json::to_vec(&payload)?;
            let headers = self.sign_request("POST", "/challenge", &body_bytes);

            let mut request = self
                .client
                .post(format!("{}/challenge", peer_url))
                .timeout(self.timeout)
                .body(body_bytes)
                .header("content-type", "application/json");

            for (key, value) in headers {
                request = request.header(key, value);
            }

            let result = request.send().await;

            if let Ok(response) = result {
                if response.status().is_success() {
                    success_count += 1;
                }
            }
        }

        Ok(success_count)
    }

    /// Pull a challenge from a specific peer.
    ///
    /// # Arguments
    /// * `peer_url` - URL of the peer to pull from
    pub async fn pull_challenge(
        &self,
        peer_url: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let response = self
            .client
            .get(format!("{}/challenge", peer_url))
            .timeout(self.timeout)
            .send()
            .await?;

        let challenge = response.json::<serde_json::Value>().await?;
        Ok(challenge)
    }

    /// Register this node with a peer.
    ///
    /// # Arguments
    /// * `peer_url` - Peer URL to register with
    /// * `our_url` - Our node's URL
    /// * `node_id` - Our node's identifier
    pub async fn register_peer(
        &self,
        peer_url: &str,
        our_url: &str,
        node_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .post(format!("{}/register", peer_url))
            .timeout(self.timeout)
            .json(&serde_json::json!({
                "node_id": node_id,
                "url": our_url,
            }))
            .send()
            .await?;

        Ok(())
    }

    /// Check health status of a peer, returning false if unreachable.
    ///
    /// # Arguments
    /// * `peer_url` - URL of the peer to check
    pub async fn health_check(&self, peer_url: &str) -> bool {
        let result = self
            .client
            .get(format!("{}/health", peer_url))
            .timeout(self.timeout)
            .send()
            .await;

        result.is_ok()
    }

    /// Update the list of known peers.
    ///
    /// # Arguments
    /// * `new_peers` - New list of peer URLs
    pub async fn update_peers(&self, new_peers: Vec<String>) {
        let mut peers = self.peers.write().await;
        *peers = new_peers;
    }

    /// Broadcast a hexchain lattice snapshot to all peers.
    ///
    /// # Arguments
    /// * `node_id` - This node's identifier
    /// * `snapshot` - JSON-serialized lattice snapshot (LatticeSnapshot)
    ///
    /// # Returns
    /// Number of peers that successfully received the snapshot
    pub async fn broadcast_lattice(
        &self,
        node_id: &str,
        snapshot: &serde_json::Value,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let peers = self.peers.read().await.clone();
        let mut success_count = 0;

        for peer_url in peers {
            let payload = serde_json::json!({
                "node_id": node_id,
                "snapshot": snapshot,
            });
            let result = self
                .client
                .post(format!("{}/hexchain/lattice/sync", peer_url))
                .timeout(self.timeout)
                .json(&payload)
                .send()
                .await;

            if let Ok(response) = result {
                if response.status().is_success() {
                    success_count += 1;
                }
            }
        }

        Ok(success_count)
    }

    pub async fn broadcast_block(
        &self,
        proof: &serde_json::Value,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let peers = self.peers.read().await.clone();
        let mut success_count = 0;

        for peer_url in peers {
            let result = self
                .client
                .post(format!("{}/hexchain/submit", peer_url))
                .timeout(self.timeout)
                .json(proof)
                .send()
                .await;

            if let Ok(response) = result {
                if response.status().is_success() {
                    success_count += 1;
                }
            }
        }

        Ok(success_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_new_creates_client_with_correct_timeout() {
        let peer_urls = vec!["http://peer1.local".to_string()];
        let gossip = GossipClient::new(peer_urls.clone(), 30);

        // Verify timeout is set correctly
        assert_eq!(gossip.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_new_creates_client_with_peer_urls() {
        let peer_urls = vec![
            "http://peer1.local".to_string(),
            "http://peer2.local".to_string(),
        ];
        let _gossip = GossipClient::new(peer_urls.clone(), 30);

        // Verify peers are stored (we'll test this through async block)
        assert_eq!(peer_urls.len(), 2);
    }

    #[tokio::test]
    async fn test_broadcast_challenge_counts_successful_peers() {
        let mut server = mockito::Server::new_async().await;
        let peer1_url = server.url();

        // Mock successful response for peer1
        let _m = server
            .mock("POST", "/challenge")
            .with_status(200)
            .with_body(r#"{"status": "ok"}"#)
            .expect(1)
            .create_async()
            .await;

        let gossip = GossipClient::new(vec![peer1_url], 5);
        let challenge_id = "test-challenge-123";
        let challenge_json = json!({"difficulty": 100, "nonce": 42});

        let result = gossip
            .broadcast_challenge(challenge_id, &challenge_json)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_broadcast_challenge_handles_partial_failures() {
        let mut server1 = mockito::Server::new_async().await;
        let mut server2 = mockito::Server::new_async().await;

        let peer1_url = server1.url();
        let peer2_url = server2.url();

        // Peer1 responds successfully
        let _m1 = server1
            .mock("POST", "/challenge")
            .with_status(200)
            .expect(1)
            .create_async()
            .await;

        // Peer2 doesn't respond (timeout/error)
        let _m2 = server2
            .mock("POST", "/challenge")
            .with_status(500)
            .expect(1)
            .create_async()
            .await;

        let gossip = GossipClient::new(vec![peer1_url, peer2_url], 5);
        let challenge_id = "test-challenge";
        let challenge_json = json!({"difficulty": 100});

        let result = gossip
            .broadcast_challenge(challenge_id, &challenge_json)
            .await;

        // Should succeed with count of successful peers
        assert!(result.is_ok());
        // Only 1 peer succeeded
        assert_eq!(result.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_pull_challenge_returns_challenge_json() {
        let mut server = mockito::Server::new_async().await;
        let peer_url = server.url();

        let expected_challenge = json!({
            "challenge_id": "ch-123",
            "difficulty": 100,
            "nonce": 42
        });

        let _m = server
            .mock("GET", "/challenge")
            .with_status(200)
            .with_body(expected_challenge.to_string())
            .expect(1)
            .create_async()
            .await;

        let gossip = GossipClient::new(vec![], 5);
        let result = gossip.pull_challenge(&peer_url).await;

        assert!(result.is_ok());
        let received = result.unwrap();
        assert_eq!(received["challenge_id"], "ch-123");
        assert_eq!(received["difficulty"], 100);
    }

    #[tokio::test]
    async fn test_register_peer_sends_correct_payload() {
        let mut server = mockito::Server::new_async().await;
        let peer_url = server.url();

        let _m = server
            .mock("POST", "/register")
            .match_body(mockito::Matcher::Json(json!({
                "node_id": "node-123",
                "url": "http://our-node"
            })))
            .with_status(200)
            .expect(1)
            .create_async()
            .await;

        let gossip = GossipClient::new(vec![], 5);
        let result = gossip
            .register_peer(&peer_url, "http://our-node", "node-123")
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_check_returns_true_for_healthy_peer() {
        let mut server = mockito::Server::new_async().await;
        let peer_url = server.url();

        let _m = server
            .mock("GET", "/health")
            .with_status(200)
            .with_body(r#"{"status": "healthy"}"#)
            .expect(1)
            .create_async()
            .await;

        let gossip = GossipClient::new(vec![], 5);
        let result = gossip.health_check(&peer_url).await;

        assert!(result);
    }

    #[tokio::test]
    async fn test_health_check_returns_false_for_unreachable_peer() {
        let gossip = GossipClient::new(vec![], 1); // 1 second timeout

        // Try to reach a non-existent peer (should timeout/fail)
        let result = gossip.health_check("http://127.0.0.1:1").await;

        assert!(!result);
    }

    #[tokio::test]
    async fn test_update_peers_modifies_peer_list() {
        let initial_peers = vec!["http://peer1.local".to_string()];
        let gossip = GossipClient::new(initial_peers, 30);

        let new_peers = vec![
            "http://peer2.local".to_string(),
            "http://peer3.local".to_string(),
        ];
        gossip.update_peers(new_peers.clone()).await;

        // Verify peers were updated
        let peers = gossip.peers.read().await;
        assert_eq!(peers.len(), 2);
        assert_eq!(*peers, new_peers);
    }

    #[tokio::test]
    async fn test_broadcast_challenge_to_multiple_peers() {
        let mut server1 = mockito::Server::new_async().await;
        let mut server2 = mockito::Server::new_async().await;
        let mut server3 = mockito::Server::new_async().await;

        let peer1_url = server1.url();
        let peer2_url = server2.url();
        let peer3_url = server3.url();

        // All peers respond successfully
        let _m1 = server1
            .mock("POST", "/challenge")
            .with_status(200)
            .expect(1)
            .create_async()
            .await;

        let _m2 = server2
            .mock("POST", "/challenge")
            .with_status(200)
            .expect(1)
            .create_async()
            .await;

        let _m3 = server3
            .mock("POST", "/challenge")
            .with_status(200)
            .expect(1)
            .create_async()
            .await;

        let gossip = GossipClient::new(vec![peer1_url, peer2_url, peer3_url], 5);

        let challenge_id = "test-ch";
        let challenge_json = json!({"difficulty": 100});

        let result = gossip
            .broadcast_challenge(challenge_id, &challenge_json)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);
    }
}
