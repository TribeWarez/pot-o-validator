use pot_o_core::TribeResult;
use pot_o_mining::Challenge;
use serde::{Deserialize, Serialize};

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
// Ed25519Authority (implemented now -- Solana keypair based)
// ---------------------------------------------------------------------------

pub struct Ed25519Authority;

impl ProofAuthority for Ed25519Authority {
    fn verify_miner_identity(&self, _pubkey: &str, _signature: &[u8]) -> TribeResult<bool> {
        // For single-node local operation, accept all identities.
        // Production: verify Ed25519 signature against pubkey.
        Ok(true)
    }

    fn sign_challenge(&self, _challenge: &Challenge) -> TribeResult<Vec<u8>> {
        // Placeholder: return empty signature for local mode.
        // Production: sign with the validator's Solana keypair.
        Ok(vec![0u8; 64])
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
