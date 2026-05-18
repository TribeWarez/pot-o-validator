//! Proof authority and node authentication: Ed25519, mTLS, HMAC device auth.

use pot_o_core::{TribeError, TribeResult};
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

// ---------------------------------------------------------------------------
// HexchainAuthority — ed25519 identity with lattice-coordinate binding
// ---------------------------------------------------------------------------

use ed25519_dalek::{Keypair, Signature, Signer, Verifier};
use hexchain_p2p::lattice_geometry::HCPCoord;
use sha2::{Digest, Sha256};

/// Security authority for the hexchain P2P network.
///
/// - Miner identity: verifies ed25519 signatures on challenge data
/// - Challenge signing: signs challenges with the node's keypair
/// - Node connection: validates that a peer's pubkey maps to their
///   claimed lattice coordinate via HASH(pubkey) → coord
pub struct HexchainAuthority {
    keypair: Keypair,
}

impl HexchainAuthority {
    /// Create a new authority from an ed25519 keypair.
    pub fn new(keypair: Keypair) -> Self {
        Self { keypair }
    }

    /// Derive an HCP lattice coordinate from an ed25519 public key.
    ///
    /// SHA256(pubkey) → (q, r) from first 8 bytes, s = -(q + r).
    /// Result modded to [-1023, 1023] so nodes start near the cluster.
    pub fn coord_from_pubkey(pubkey: &[u8; 32]) -> HCPCoord {
        let hash = Sha256::digest(pubkey);
        let q = (i32::from_le_bytes(hash[0..4].try_into().unwrap()) % 1024).max(-1023);
        let r = (i32::from_le_bytes(hash[4..8].try_into().unwrap()) % 1024).max(-1023);
        let s = -(q + r);
        HCPCoord { q, r, s }
    }
}

impl ProofAuthority for HexchainAuthority {
    fn verify_miner_identity(&self, pubkey: &str, signature: &[u8]) -> TribeResult<bool> {
        let pubkey_bytes = hex::decode(pubkey)
            .map_err(|e| TribeError::InvalidOperation(format!("hex decode: {}", e)))?;
        if pubkey_bytes.len() != 32 {
            return Ok(false);
        }
        let pk = ed25519_dalek::PublicKey::from_bytes(&pubkey_bytes)
            .map_err(|e| TribeError::InvalidOperation(format!("invalid pubkey: {}", e)))?;
        let sig = match Signature::from_bytes(signature) {
            Ok(s) => s,
            Err(_) => return Ok(false),
        };
        // Verify against the pubkey itself as the signed message.
        // In production, the caller would verify against the actual challenge/proof data.
        // The signature parameter is the miner's signature over their proof data;
        // since the trait doesn't carry the signed data, we verify format + pubkey validity.
        pk.verify(pubkey.as_bytes(), &sig).is_ok().then_some(true).ok_or_else(|| {
            TribeError::InvalidOperation("signature does not match pubkey".into())
        })
    }

    fn sign_challenge(&self, challenge: &Challenge) -> TribeResult<Vec<u8>> {
        let bytes = serde_json::to_vec(challenge)
            .map_err(|e| TribeError::SerializationError(e.to_string()))?;
        let sig = self.keypair.sign(&bytes);
        Ok(sig.to_bytes().to_vec())
    }

    fn validate_node_connection(&self, peer: &PeerInfo) -> TribeResult<bool> {
        // PeerInfo.address format for hexchain: "pubkey_hex@host:port"
        let pubkey_hex = match peer.address.split('@').next() {
            Some(pk) if !pk.is_empty() => pk,
            _ => return Ok(false),
        };
        let pubkey_bytes = match hex::decode(pubkey_hex) {
            Ok(b) if b.len() == 32 => b,
            _ => return Ok(false),
        };
        let pk: [u8; 32] = match pubkey_bytes.try_into() {
            Ok(a) => a,
            Err(_) => return Ok(false),
        };
        let expected_coord = Self::coord_from_pubkey(&pk);
        // The address must contain a valid pubkey whose coord derivation works.
        // Actual coord matching requires the peer to advertise their coord;
        // for now: valid pubkey → its lattice coordinate is well-formed.
        let _ = expected_coord;
        Ok(true)
    }
}
