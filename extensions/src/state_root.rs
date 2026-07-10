use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub fn compute_state_root(
    balances: &BTreeMap<String, u64>,
    nonces: &BTreeMap<String, u64>,
) -> [u8; 32] {
    let mut leaves: Vec<[u8; 32]> = Vec::new();

    for (key, balance) in balances {
        let mut h = Sha256::new();
        h.update(b"balance:");
        h.update(key.as_bytes());
        h.update(balance.to_le_bytes());
        let mut leaf = [0u8; 32];
        leaf.copy_from_slice(&h.finalize());
        leaves.push(leaf);
    }

    for (addr, nonce) in nonces {
        let mut h = Sha256::new();
        h.update(b"nonce:");
        h.update(addr.as_bytes());
        h.update(nonce.to_le_bytes());
        let mut leaf = [0u8; 32];
        leaf.copy_from_slice(&h.finalize());
        leaves.push(leaf);
    }

    if leaves.is_empty() {
        return [0u8; 32];
    }

    merkle_tree(&leaves)
}

fn merkle_tree(leaves: &[[u8; 32]]) -> [u8; 32] {
    let mut level = leaves.to_vec();
    while level.len() > 1 {
        let mut next = Vec::new();
        for chunk in level.chunks(2) {
            let mut h = Sha256::new();
            h.update(chunk[0]);
            h.update(if chunk.len() > 1 { chunk[1] } else { chunk[0] });
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&h.finalize());
            next.push(hash);
        }
        level = next;
    }
    level[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_root_deterministic() {
        let mut balances = BTreeMap::new();
        balances.insert("alice:TRIBE".to_string(), 1000);
        balances.insert("bob:TRIBE".to_string(), 500);

        let mut nonces = BTreeMap::new();
        nonces.insert("alice".to_string(), 1);

        let root1 = compute_state_root(&balances, &nonces);
        let root2 = compute_state_root(&balances, &nonces);
        assert_eq!(root1, root2);
    }

    #[test]
    fn test_state_root_changes_with_balance() {
        let mut balances = BTreeMap::new();
        balances.insert("alice:TRIBE".to_string(), 1000);
        let nonces = BTreeMap::new();

        let root1 = compute_state_root(&balances, &nonces);

        balances.insert("alice:TRIBE".to_string(), 2000);
        let root2 = compute_state_root(&balances, &nonces);

        assert_ne!(root1, root2);
    }

    #[test]
    fn test_empty_state_root() {
        let balances = BTreeMap::new();
        let nonces = BTreeMap::new();
        let root = compute_state_root(&balances, &nonces);
        assert_eq!(root, [0u8; 32]);
    }
}
