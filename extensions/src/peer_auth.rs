use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};

pub struct NodeIdentity {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    node_id: String,
}

impl NodeIdentity {
    pub fn new(node_id: String) -> Self {
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
            node_id,
        }
    }

    pub fn from_bytes(node_id: String, secret_bytes: [u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
            node_id,
        }
    }

    pub fn sign_message(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_bytes().to_vec()
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }
}

pub fn verify_peer_signature(peer_pubkey: &[u8; 32], message: &[u8], signature: &[u8]) -> bool {
    if signature.len() != 64 {
        return false;
    }
    let Ok(verifying_key) = VerifyingKey::from_bytes(peer_pubkey) else {
        return false;
    };
    let sig_bytes: &[u8; 64] = match signature.try_into() {
        Ok(b) => b,
        Err(_) => return false,
    };
    let sig = Signature::from_bytes(sig_bytes);
    verifying_key.verify_strict(message, &sig).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let identity = NodeIdentity::new("test-node".to_string());
        let message = b"hello world";
        let signature = identity.sign_message(message);
        assert!(verify_peer_signature(
            &identity.public_key_bytes(),
            message,
            &signature
        ));
    }

    #[test]
    fn test_verify_rejects_tampered_message() {
        let identity = NodeIdentity::new("test-node".to_string());
        let signature = identity.sign_message(b"hello");
        assert!(!verify_peer_signature(
            &identity.public_key_bytes(),
            b"world",
            &signature
        ));
    }

    #[test]
    fn test_from_bytes_roundtrip() {
        let identity = NodeIdentity::new("node-a".to_string());
        let secret_bytes = identity.signing_key.to_bytes();
        let restored = NodeIdentity::from_bytes("node-a".to_string(), secret_bytes);
        assert_eq!(restored.public_key_bytes(), identity.public_key_bytes());
        let msg = b"test message";
        let sig = restored.sign_message(msg);
        assert!(verify_peer_signature(
            &identity.public_key_bytes(),
            msg,
            &sig
        ));
    }

    #[test]
    fn test_verify_rejects_wrong_key() {
        let identity_a = NodeIdentity::new("a".to_string());
        let identity_b = NodeIdentity::new("b".to_string());
        let msg = b"hello";
        let sig = identity_a.sign_message(msg);
        assert!(!verify_peer_signature(
            &identity_b.public_key_bytes(),
            msg,
            &sig
        ));
    }

    #[test]
    fn test_verify_rejects_short_signature() {
        let identity = NodeIdentity::new("test".to_string());
        assert!(!verify_peer_signature(
            &identity.public_key_bytes(),
            b"msg",
            &[0u8; 32]
        ));
    }
}
