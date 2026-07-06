use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

const SESSION_TTL: Duration = Duration::from_secs(24 * 3600);
const CHALLENGE_TTL: Duration = Duration::from_secs(120);

#[allow(dead_code)]
pub struct AuthChallenge {
    pub pubkey: String,
    pub nonce: [u8; 32],
    pub created_at: Instant,
}

pub struct Session {
    pub pubkey: String,
    #[allow(dead_code)]
    pub token: String,
    pub created_at: Instant,
}

pub struct AuthState {
    challenges: Arc<RwLock<HashMap<String, AuthChallenge>>>,
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    server_secret: [u8; 32],
}

impl Default for AuthState {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthState {
    pub fn new() -> Self {
        let mut secret = [0u8; 32];
        use rand::RngCore;
        OsRng.fill_bytes(&mut secret);
        Self {
            challenges: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            server_secret: secret,
        }
    }

    pub async fn create_challenge(&self, pubkey: &str) -> [u8; 32] {
        let mut nonce = [0u8; 32];
        use rand::RngCore;
        OsRng.fill_bytes(&mut nonce);

        let challenge = AuthChallenge {
            pubkey: pubkey.to_string(),
            nonce,
            created_at: Instant::now(),
        };

        self.challenges
            .write()
            .await
            .insert(pubkey.to_string(), challenge);
        nonce
    }

    pub async fn verify_challenge(
        &self,
        pubkey: &str,
        signature: &[u8],
        message: &[u8],
    ) -> Result<String, String> {
        let challenge = self
            .challenges
            .write()
            .await
            .remove(pubkey)
            .ok_or_else(|| "No challenge found for this pubkey".to_string())?;

        if challenge.created_at.elapsed() > CHALLENGE_TTL {
            return Err("Challenge expired".to_string());
        }

        let pubkey_bytes: [u8; 32] = hex::decode(pubkey)
            .map_err(|e| format!("Invalid pubkey hex: {}", e))?
            .try_into()
            .map_err(|_| "Invalid pubkey length".to_string())?;
        let public_key = ed25519_dalek::PublicKey::from_bytes(&pubkey_bytes)
            .map_err(|e| format!("Invalid pubkey: {}", e))?;

        let sig_bytes: [u8; 64] = signature
            .try_into()
            .map_err(|_| "Invalid signature length".to_string())?;
        let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes)
            .map_err(|e| format!("Invalid signature: {}", e))?;

        public_key
            .verify_strict(message, &signature)
            .map_err(|_| "Signature verification failed".to_string())?;

        let token = self.issue_token(pubkey);
        Ok(token)
    }

    fn issue_token(&self, pubkey: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.server_secret);
        hasher.update(pubkey.as_bytes());
        hasher.update(Instant::now().elapsed().as_nanos().to_le_bytes());
        let token = hex::encode(hasher.finalize());

        let session = Session {
            pubkey: pubkey.to_string(),
            token: token.clone(),
            created_at: Instant::now(),
        };

        let mut sessions = self.sessions.blocking_write();
        sessions.insert(token.clone(), session);

        token
    }

    pub async fn validate_token(&self, token: &str) -> Option<String> {
        self.sweep_expired().await;
        let sessions = self.sessions.read().await;
        sessions
            .get(token)
            .filter(|s| s.created_at.elapsed() < SESSION_TTL)
            .map(|s| s.pubkey.clone())
    }

    async fn sweep_expired(&self) {
        let mut sessions = self.sessions.write().await;
        sessions.retain(|_, s| s.created_at.elapsed() < SESSION_TTL);
    }
}
