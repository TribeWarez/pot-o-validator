use sha2::{Digest, Sha256};

pub struct MerkleProof {
    pub tx_hash: [u8; 32],
    pub merkle_root: [u8; 32],
    pub proof: Vec<[u8; 32]>,
    pub index: usize,
}

pub fn generate_merkle_proof(
    transactions: &[serde_json::Value],
    tx_index: usize,
) -> Option<MerkleProof> {
    if tx_index >= transactions.len() {
        return None;
    }

    let tx_data = serde_json::to_string(&transactions[tx_index]).unwrap_or_default();
    let tx_hash: [u8; 32] = Sha256::digest(tx_data.as_bytes()).into();

    let leaves: Vec<[u8; 32]> = transactions
        .iter()
        .map(|tx| {
            let data = serde_json::to_string(tx).unwrap_or_default();
            Sha256::digest(data.as_bytes()).into()
        })
        .collect();

    let merkle_root = compute_merkle_root(&leaves);
    let proof = build_merkle_branch(&leaves, tx_index);

    Some(MerkleProof {
        tx_hash,
        merkle_root,
        proof,
        index: tx_index,
    })
}

pub fn verify_merkle_proof(proof: &MerkleProof) -> bool {
    let mut current = proof.tx_hash;
    let mut index = proof.index;

    for sibling in &proof.proof {
        current = if index.is_multiple_of(2) {
            hash_pair(current, *sibling)
        } else {
            hash_pair(*sibling, current)
        };
        index /= 2;
    }

    current == proof.merkle_root
}

fn hash_pair(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(left);
    h.update(right);
    h.finalize().into()
}

fn compute_merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    let mut level = leaves.to_vec();
    while level.len() > 1 {
        let mut next = Vec::new();
        for chunk in level.chunks(2) {
            next.push(hash_pair(
                chunk[0],
                if chunk.len() > 1 { chunk[1] } else { chunk[0] },
            ));
        }
        level = next;
    }
    level[0]
}

fn build_merkle_branch(leaves: &[[u8; 32]], index: usize) -> Vec<[u8; 32]> {
    let mut proof = Vec::new();
    let mut level = leaves.to_vec();
    let mut idx = index;

    while level.len() > 1 {
        let sibling_idx = if idx.is_multiple_of(2) {
            idx + 1
        } else {
            idx - 1
        };
        if sibling_idx < level.len() {
            proof.push(level[sibling_idx]);
        }

        let mut next = Vec::new();
        for chunk in level.chunks(2) {
            next.push(hash_pair(
                chunk[0],
                if chunk.len() > 1 { chunk[1] } else { chunk[0] },
            ));
        }
        level = next;
        idx /= 2;
    }

    proof
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_proof_generation_and_verification() {
        let txs = vec![
            serde_json::json!({"from": "alice", "to": "bob", "amount": 100}),
            serde_json::json!({"from": "bob", "to": "charlie", "amount": 50}),
            serde_json::json!({"from": "charlie", "to": "alice", "amount": 25}),
        ];

        let proof = generate_merkle_proof(&txs, 1).unwrap();
        assert!(verify_merkle_proof(&proof));
    }

    #[test]
    fn test_invalid_proof_fails_verification() {
        let txs = vec![serde_json::json!({"tx": 1})];
        let mut proof = generate_merkle_proof(&txs, 0).unwrap();
        proof.tx_hash = [0xFF; 32];
        assert!(!verify_merkle_proof(&proof));
    }
}
