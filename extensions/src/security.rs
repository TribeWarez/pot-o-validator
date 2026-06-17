//! Proof authority and node authentication: Ed25519, mTLS, HMAC device auth.

use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer};
use pot_o_core::TribeResult;
use pot_o_mining::Challenge;

use crate::peer_network::PeerInfo;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Security layer for proof submission and node authentication.
pub trait ProofAuthority: Send + Sync {
    fn verify_miner_identity(&self, pubkey: &str, signature: &[u8]) -> TribeResult<bool>;
    fn sign_challenge(&self, challenge: &Challenge) -> TribeResult<Vec<u8>>;
    fn validate_node_connection(&self, peer: &PeerInfo) -> TribeResult<bool>;
}

// ---------------------------------------------------------------------------
// Ed25519Authority (real Ed25519 signing via ed25519-dalek)
// ---------------------------------------------------------------------------

/// Ed25519 authority that signs challenges with the validator's keypair.
///
/// If no keypair is loaded (e.g., file not found), signing falls back to
/// returning a zero-filled signature — acceptable for local/test mode.
pub struct Ed25519Authority {
    keypair: Option<Keypair>,
}

impl Ed25519Authority {
    /// Load the signing key from a Solana-format keypair file.
    ///
    /// The file is a JSON array of 64 integers: first 32 bytes = secret seed,
    /// next 32 bytes = public key.  If the file cannot be read, signing will
    /// be unavailable (returns zero signatures) but construction does not fail.
    pub fn new(keypair_path: &str) -> Self {
        let contents = match std::fs::read_to_string(keypair_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    path = %keypair_path,
                    error = %e,
                    "Ed25519Authority: cannot read keypair file; signing disabled"
                );
                return Self { keypair: None };
            }
        };

        let keypair_bytes: Vec<u8> = match serde_json::from_str(&contents) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(
                    path = %keypair_path,
                    error = %e,
                    "Ed25519Authority: invalid JSON keypair format; signing disabled"
                );
                return Self { keypair: None };
            }
        };

        if keypair_bytes.len() >= 32 {
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&keypair_bytes[..32]);
            match SecretKey::from_bytes(&seed) {
                Ok(secret) => {
                    let public = PublicKey::from(&secret);
                    let keypair = Keypair { secret, public };
                    tracing::info!(
                        pubkey = %hex::encode(public.to_bytes()),
                        "Ed25519Authority: loaded signing key"
                    );
                    Self {
                        keypair: Some(keypair),
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        path = %keypair_path,
                        error = %e,
                        "Ed25519Authority: invalid secret key bytes; signing disabled"
                    );
                    Self { keypair: None }
                }
            }
        } else {
            tracing::warn!(
                path = %keypair_path,
                "Ed25519Authority: keypair file too short ({} bytes); signing disabled",
                keypair_bytes.len()
            );
            Self { keypair: None }
        }
    }

    fn public_key_from_str(pubkey: &str) -> Option<PublicKey> {
        let bytes = hex::decode(pubkey).ok()?;
        let arr: [u8; 32] = bytes.try_into().ok()?;
        PublicKey::from_bytes(&arr).ok()
    }
}

impl ProofAuthority for Ed25519Authority {
    fn verify_miner_identity(&self, pubkey: &str, signature: &[u8]) -> TribeResult<bool> {
        let pk = match Self::public_key_from_str(pubkey) {
            Some(pk) => pk,
            None => return Ok(false),
        };
        let sig = match Signature::from_bytes(signature) {
            Ok(s) => s,
            Err(_) => return Ok(false),
        };
        // We verify against the pubkey itself (the message is the pubkey bytes,
        // proving the miner controls the corresponding secret key)
        Ok(pk.verify_strict(pubkey.as_bytes(), &sig).is_ok())
    }

    fn sign_challenge(&self, challenge: &Challenge) -> TribeResult<Vec<u8>> {
        match &self.keypair {
            Some(kp) => {
                let msg = serde_json::to_vec(challenge)
                    .map_err(|e| pot_o_core::TribeError::SerializationError(e.to_string()))?;
                let sig = kp
                    .try_sign(&msg)
                    .map_err(|e| pot_o_core::TribeError::SerializationError(e.to_string()))?;
                Ok(sig.to_bytes().to_vec())
            }
            None => {
                tracing::warn!("Ed25519Authority: no signing key loaded; returning zero signature");
                Ok(vec![0u8; 64])
            }
        }
    }

    fn validate_node_connection(&self, _peer: &PeerInfo) -> TribeResult<bool> {
        // Single-node: no peer validation needed.
        Ok(true)
    }
}

// ---------------------------------------------------------------------------
// MtlsAuthority (stubbed -- for VPN node-to-node auth)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MtlsConfig {
    pub ca_cert_path: String,
    pub node_cert_path: String,
    pub node_key_path: String,
}

pub struct MtlsAuthority {
    pub config: MtlsConfig,
}

impl ProofAuthority for MtlsAuthority {
    fn verify_miner_identity(&self, _pubkey: &str, _signature: &[u8]) -> TribeResult<bool> {
        todo!("mTLS miner identity verification not yet implemented")
    }
    fn sign_challenge(&self, _challenge: &Challenge) -> TribeResult<Vec<u8>> {
        todo!("mTLS challenge signing not yet implemented")
    }
    fn validate_node_connection(&self, _peer: &PeerInfo) -> TribeResult<bool> {
        todo!("mTLS node connection validation not yet implemented")
    }
}

// ---------------------------------------------------------------------------
// HmacDeviceAuth (stubbed -- shared-secret HMAC for ESP devices)
// ---------------------------------------------------------------------------

pub struct HmacDeviceAuth {
    pub shared_secret: Vec<u8>,
}

impl ProofAuthority for HmacDeviceAuth {
    fn verify_miner_identity(&self, _pubkey: &str, _signature: &[u8]) -> TribeResult<bool> {
        todo!("HMAC device identity verification not yet implemented")
    }
    fn sign_challenge(&self, _challenge: &Challenge) -> TribeResult<Vec<u8>> {
        todo!("HMAC challenge signing not yet implemented")
    }
    fn validate_node_connection(&self, _peer: &PeerInfo) -> TribeResult<bool> {
        todo!("HMAC node connection validation not yet implemented")
    }
}
