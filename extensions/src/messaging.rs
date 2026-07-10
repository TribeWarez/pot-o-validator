use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};

/// Message sent from a miner to the validator over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MinerMessage {
    Subscribe {
        device_id: String,
        device_type: String,
    },
    Unsubscribe {
        device_id: String,
    },
    Heartbeat {
        device_id: String,
    },
    SubmitProof {
        device_id: String,
        challenge_id: String,
        proof_hex: String,
        signature: String,
        device_type: String,
    },
    Progress {
        device_id: String,
        challenge_id: String,
        hash: String,
        progress_pct: f64,
    },
}

/// Message sent from the validator to a miner over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ValidatorMessage {
    Challenge {
        challenge_json: String,
    },
    ProofAccepted {
        tx_signature: String,
    },
    ProofRejected {
        reason: String,
    },
    HeartbeatAck,
    Error {
        code: String,
        message: String,
    },
    Subscribed {
        device_id: String,
    },
    NewBlock {
        height: u64,
        hash: String,
        tx_count: usize,
        timestamp: u64,
    },
}

impl ValidatorMessage {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"type":"error","code":"serialization_failed","message":"internal error"}"#.into()
        })
    }
}

/// Connection manager for miner WebSocket sessions.
/// Each connected miner gets an `mpsc::UnboundedSender` that the WS task polls.
pub struct Messaging {
    connections: RwLock<HashMap<String, mpsc::UnboundedSender<String>>>,
}

impl Messaging {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            connections: RwLock::new(HashMap::new()),
        })
    }

    /// Register a connected miner.
    pub async fn register(&self, device_id: String, tx: mpsc::UnboundedSender<String>) {
        self.connections.write().await.insert(device_id, tx);
    }

    /// Remove a disconnected miner.
    pub async fn unregister(&self, device_id: &str) {
        self.connections.write().await.remove(device_id);
    }

    /// Number of currently connected miners.
    pub async fn connected_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// List all connected device IDs.
    pub async fn active_devices(&self) -> Vec<String> {
        self.connections.read().await.keys().cloned().collect()
    }

    /// Push a message to every connected miner.
    pub async fn broadcast(&self, msg: &ValidatorMessage) {
        let json = msg.to_json();
        let mut dead = Vec::new();
        for (id, tx) in self.connections.read().await.iter() {
            if tx.send(json.clone()).is_err() {
                dead.push(id.clone());
            }
        }
        if !dead.is_empty() {
            let mut w = self.connections.write().await;
            for id in dead {
                w.remove(&id);
            }
        }
    }

    /// Push a message to a specific miner.
    pub async fn send_to(&self, device_id: &str, msg: &ValidatorMessage) {
        let json = msg.to_json();
        if let Some(tx) = self.connections.read().await.get(device_id) {
            if tx.send(json).is_err() {
                self.unregister(device_id).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_messaging_register_and_count() {
        let m = Messaging::new();
        let (tx, _) = mpsc::unbounded_channel();
        m.register("miner-1".into(), tx).await;
        assert_eq!(m.connected_count().await, 1);
        assert_eq!(m.active_devices().await, vec!["miner-1".to_string()]);
    }

    #[tokio::test]
    async fn test_messaging_unregister() {
        let m = Messaging::new();
        let (tx, _) = mpsc::unbounded_channel();
        m.register("miner-1".into(), tx).await;
        m.unregister("miner-1").await;
        assert_eq!(m.connected_count().await, 0);
    }

    #[tokio::test]
    async fn test_messaging_broadcast() {
        let m = Messaging::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        m.register("miner-1".into(), tx).await;

        m.broadcast(&ValidatorMessage::HeartbeatAck).await;
        let received: String = rx.recv().await.unwrap();
        assert!(received.contains("\"type\":\"heartbeat_ack\""));
    }

    #[tokio::test]
    async fn test_messaging_send_to() {
        let m = Messaging::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        m.register("miner-1".into(), tx).await;

        m.send_to("miner-1", &ValidatorMessage::HeartbeatAck).await;
        let received: String = rx.recv().await.unwrap();
        assert!(received.contains("\"type\":\"heartbeat_ack\""));
    }

    #[tokio::test]
    async fn test_messaging_cleanup_dead() {
        let m = Messaging::new();
        let (tx, rx) = mpsc::unbounded_channel();
        m.register("miner-1".into(), tx).await;
        drop(rx); // drop receiver to simulate disconnect

        // Broadcast should detect dead connection and clean up
        m.broadcast(&ValidatorMessage::HeartbeatAck).await;
        assert_eq!(m.connected_count().await, 0);
    }

    #[tokio::test]
    async fn test_validator_message_to_json() {
        let msg = ValidatorMessage::HeartbeatAck;
        let json = msg.to_json();
        assert!(json.contains("\"type\":\"heartbeat_ack\""));

        let msg = ValidatorMessage::ProofAccepted {
            tx_signature: "sig123".into(),
        };
        let json = msg.to_json();
        assert!(json.contains("proof_accepted"));
        assert!(json.contains("sig123"));
    }

    #[test]
    fn test_miner_message_serialization() {
        let msg = MinerMessage::Subscribe {
            device_id: "esp-01".into(),
            device_type: "esp32s".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"subscribe\""));
        assert!(json.contains("esp-01"));

        let msg = MinerMessage::Heartbeat {
            device_id: "esp-01".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"heartbeat\""));
    }
}
