use ed25519_dalek::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

pub use pot_o_core::TokenType;

/// Stable u8 discriminant for TokenType (used in hash to avoid Display instability).
pub fn token_discriminant(t: &TokenType) -> u8 {
    match t {
        TokenType::TribeChain => 0,
        TokenType::PTtC => 1,
        TokenType::NMTC => 2,
        TokenType::STOMP => 3,
        TokenType::AUM => 4,
        TokenType::AI3 => 5,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferTransaction {
    pub tx_hash: [u8; 32],
    pub nonce: u64,
    pub from: String,
    pub to: String,
    pub token: TokenType,
    pub amount: u64,
    pub fee: u64,
    pub signature: Vec<u8>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofRewardEntry {
    pub miner_pubkey: String,
    pub reward_amount: u64,
    pub proof_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoinbaseTransaction {
    pub tx_hash: [u8; 32],
    pub height: u64,
    pub miner_address: String,
    pub block_reward: u64,
    pub proof_rewards: Vec<ProofRewardEntry>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transaction {
    Coinbase(CoinbaseTransaction),
    Transfer(TransferTransaction),
}

impl Transaction {
    pub fn tx_hash(&self) -> &[u8; 32] {
        match self {
            Transaction::Coinbase(tx) => &tx.tx_hash,
            Transaction::Transfer(tx) => &tx.tx_hash,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxError {
    InsufficientBalance,
    InvalidNonce,
    InvalidSignature,
    DuplicateTransaction,
    InvalidToken,
    AmountZero,
    SelfTransfer,
    FeeTooLow,
    CoinbaseNotFirst,
    CoinbaseWrongToken,
    CoinbaseRewardMismatch,
    SupplyCapExceeded,
    CoinbaseImmature,
    MerkleRootMismatch,
}

impl fmt::Display for TxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TxError::InsufficientBalance => write!(f, "insufficient balance"),
            TxError::InvalidNonce => write!(f, "invalid nonce"),
            TxError::InvalidSignature => write!(f, "invalid signature"),
            TxError::DuplicateTransaction => write!(f, "duplicate transaction"),
            TxError::InvalidToken => write!(f, "invalid token"),
            TxError::AmountZero => write!(f, "amount must be greater than zero"),
            TxError::SelfTransfer => write!(f, "self-transfer not allowed"),
            TxError::FeeTooLow => write!(f, "fee below minimum"),
            TxError::CoinbaseNotFirst => write!(f, "coinbase transaction must be first in block"),
            TxError::CoinbaseWrongToken => {
                write!(f, "coinbase transaction must use TRIBECHAIN token")
            }
            TxError::CoinbaseRewardMismatch => {
                write!(f, "coinbase reward does not match expected block reward")
            }
            TxError::SupplyCapExceeded => write!(f, "supply cap exceeded"),
            TxError::CoinbaseImmature => write!(f, "coinbase reward is not yet spendable"),
            TxError::MerkleRootMismatch => write!(f, "transaction merkle root mismatch"),
        }
    }
}

impl std::error::Error for TxError {}

pub fn hash_transfer(
    from: &str,
    nonce: u64,
    to: &str,
    token: &TokenType,
    amount: u64,
    fee: u64,
    timestamp: u64,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(from.as_bytes());
    hasher.update(nonce.to_le_bytes());
    hasher.update(to.as_bytes());
    hasher.update([token_discriminant(token)]);
    hasher.update(amount.to_le_bytes());
    hasher.update(fee.to_le_bytes());
    hasher.update(timestamp.to_le_bytes());
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

pub fn hash_coinbase(
    height: u64,
    miner_address: &str,
    block_reward: u64,
    proof_rewards: &[ProofRewardEntry],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(height.to_le_bytes());
    hasher.update(miner_address.as_bytes());
    hasher.update(block_reward.to_le_bytes());
    hasher.update((proof_rewards.len() as u64).to_le_bytes());
    for pr in proof_rewards {
        hasher.update(pr.miner_pubkey.as_bytes());
        hasher.update(pr.reward_amount.to_le_bytes());
        hasher.update(pr.proof_hash.as_bytes());
    }
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

pub fn verify_transfer_sig(tx: &TransferTransaction) -> Result<(), TxError> {
    let pubkey_bytes = bs58::decode(&tx.from)
        .into_vec()
        .map_err(|_| TxError::InvalidSignature)?;
    let pubkey_array: [u8; 32] = pubkey_bytes
        .try_into()
        .map_err(|_| TxError::InvalidSignature)?;
    let public_key = PublicKey::from_bytes(&pubkey_array).map_err(|_| TxError::InvalidSignature)?;
    let sig = Signature::from_bytes(&tx.signature).map_err(|_| TxError::InvalidSignature)?;

    let expected_hash = hash_transfer(
        &tx.from,
        tx.nonce,
        &tx.to,
        &tx.token,
        tx.amount,
        tx.fee,
        tx.timestamp,
    );

    tracing::debug!(
        from = %tx.from,
        nonce = tx.nonce,
        to = %tx.to,
        token = ?tx.token,
        amount = tx.amount,
        fee = tx.fee,
        timestamp = tx.timestamp,
        expected_hash = %hex::encode(expected_hash),
        sig_from_tx = %hex::encode(&tx.signature[..]),
        "verify_transfer_sig computing expected_hash"
    );

    public_key
        .verify_strict(&expected_hash[..], &sig)
        .map_err(|_| TxError::InvalidSignature)
}

pub fn verify_coinbase_sig(cb: &CoinbaseTransaction) -> Result<(), TxError> {
    let pubkey_bytes = bs58::decode(&cb.miner_address)
        .into_vec()
        .map_err(|_| TxError::InvalidSignature)?;
    let pubkey_array: [u8; 32] = pubkey_bytes
        .try_into()
        .map_err(|_| TxError::InvalidSignature)?;
    let public_key = PublicKey::from_bytes(&pubkey_array).map_err(|_| TxError::InvalidSignature)?;
    let sig = Signature::from_bytes(&cb.signature).map_err(|_| TxError::InvalidSignature)?;

    let expected_hash = hash_coinbase(
        cb.height,
        &cb.miner_address,
        cb.block_reward,
        &cb.proof_rewards,
    );
    public_key
        .verify_strict(&expected_hash[..], &sig)
        .map_err(|_| TxError::InvalidSignature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Keypair, SecretKey, Signer};

    #[test]
    fn test_token_discriminant() {
        assert_eq!(token_discriminant(&TokenType::TribeChain), 0);
        assert_eq!(token_discriminant(&TokenType::PTtC), 1);
        assert_eq!(token_discriminant(&TokenType::NMTC), 2);
        assert_eq!(token_discriminant(&TokenType::STOMP), 3);
        assert_eq!(token_discriminant(&TokenType::AUM), 4);
        assert_eq!(token_discriminant(&TokenType::AI3), 5);
    }

    #[test]
    fn test_tx_error_display() {
        assert_eq!(
            TxError::InsufficientBalance.to_string(),
            "insufficient balance"
        );
        assert_eq!(TxError::InvalidNonce.to_string(), "invalid nonce");
        assert_eq!(
            TxError::AmountZero.to_string(),
            "amount must be greater than zero"
        );
        assert_eq!(
            TxError::CoinbaseImmature.to_string(),
            "coinbase reward is not yet spendable"
        );
        assert_eq!(
            TxError::MerkleRootMismatch.to_string(),
            "transaction merkle root mismatch"
        );
    }

    #[test]
    fn test_hash_transfer_deterministic() {
        let h1 = hash_transfer("Alice", 1, "Bob", &TokenType::TribeChain, 100, 1, 0);
        let h2 = hash_transfer("Alice", 1, "Bob", &TokenType::TribeChain, 100, 1, 0);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 32);
    }

    #[test]
    fn test_hash_transfer_different_nonce() {
        let h1 = hash_transfer("Alice", 1, "Bob", &TokenType::TribeChain, 100, 1, 0);
        let h2 = hash_transfer("Alice", 2, "Bob", &TokenType::TribeChain, 100, 1, 0);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_coinbase_deterministic() {
        let pr = vec![];
        let h1 = hash_coinbase(42, "MinerPubkey", 500, &pr);
        let h2 = hash_coinbase(42, "MinerPubkey", 500, &pr);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 32);
    }

    #[test]
    fn test_hash_coinbase_different_height() {
        let pr = vec![];
        let h1 = hash_coinbase(1, "Miner", 500, &pr);
        let h2 = hash_coinbase(2, "Miner", 500, &pr);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_verify_transfer_sig_valid() {
        let seed = [42u8; 32];
        let secret = SecretKey::from_bytes(&seed).unwrap();
        let public = PublicKey::from(&secret);
        let keypair = Keypair { secret, public };

        let from = bs58::encode(keypair.public.to_bytes()).into_string();
        let nonce = 1u64;
        let to = bs58::encode([99u8; 32]).into_string();
        let token = TokenType::TribeChain;
        let amount = 100u64;
        let fee = 1u64;
        let timestamp = 0u64;

        let msg = hash_transfer(&from, nonce, &to, &token, amount, fee, timestamp);
        let signature = keypair.sign(&msg).to_bytes().to_vec();

        let tx = TransferTransaction {
            tx_hash: msg,
            nonce,
            from,
            to,
            token,
            amount,
            fee,
            signature,
            timestamp,
        };
        let result = verify_transfer_sig(&tx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_transfer_sig_bad_key() {
        let from = bs58::encode([0u8; 32]).into_string();
        let signature = vec![0u8; 64];
        let tx = TransferTransaction {
            tx_hash: [0u8; 32],
            nonce: 1,
            from,
            to: "to".to_string(),
            token: TokenType::TribeChain,
            amount: 100,
            fee: 1,
            signature,
            timestamp: 0,
        };
        let result = verify_transfer_sig(&tx);
        assert_eq!(result, Err(TxError::InvalidSignature));
    }

    #[test]
    fn test_verify_transfer_sig_tampered_message() {
        let seed = [42u8; 32];
        let secret = SecretKey::from_bytes(&seed).unwrap();
        let public = PublicKey::from(&secret);
        let keypair = Keypair { secret, public };

        let from = bs58::encode(keypair.public.to_bytes()).into_string();
        let nonce = 1u64;
        let to = bs58::encode([99u8; 32]).into_string();
        let token = TokenType::TribeChain;
        let amount = 100u64;
        let fee = 1u64;
        let timestamp = 0u64;

        let msg = hash_transfer(&from, nonce, &to, &token, amount, fee, timestamp);
        let signature = keypair.sign(&msg).to_bytes().to_vec();

        let tampered_amount = amount + 1;
        let tx = TransferTransaction {
            tx_hash: hash_transfer(&from, nonce, &to, &token, tampered_amount, fee, timestamp),
            nonce,
            from,
            to,
            token,
            amount: tampered_amount,
            fee,
            signature,
            timestamp: 0,
        };
        let result = verify_transfer_sig(&tx);
        assert_eq!(result, Err(TxError::InvalidSignature));
    }

    #[test]
    fn test_verify_coinbase_sig_valid() {
        let seed = [99u8; 32];
        let secret = SecretKey::from_bytes(&seed).unwrap();
        let public = PublicKey::from(&secret);
        let keypair = Keypair { secret, public };

        let miner_address = bs58::encode(keypair.public.to_bytes()).into_string();
        let height = 42u64;
        let block_reward = 500u64;
        let proof_rewards = vec![ProofRewardEntry {
            miner_pubkey: "miner1".to_string(),
            reward_amount: 100,
            proof_hash: "abc123".to_string(),
        }];

        let msg = hash_coinbase(height, &miner_address, block_reward, &proof_rewards);
        let signature = keypair.sign(&msg).to_bytes().to_vec();

        let cb = CoinbaseTransaction {
            tx_hash: msg,
            height,
            miner_address,
            block_reward,
            proof_rewards,
            signature,
        };
        let result = verify_coinbase_sig(&cb);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_coinbase_sig_invalid_pubkey() {
        let miner_address = bs58::encode([0u8; 32]).into_string();
        let signature = vec![0u8; 64];
        let cb = CoinbaseTransaction {
            tx_hash: [0u8; 32],
            height: 42,
            miner_address,
            block_reward: 500,
            proof_rewards: vec![],
            signature,
        };
        let result = verify_coinbase_sig(&cb);
        assert_eq!(result, Err(TxError::InvalidSignature));
    }

    #[test]
    fn test_verify_coinbase_sig_tampered_reward() {
        let seed = [99u8; 32];
        let secret = SecretKey::from_bytes(&seed).unwrap();
        let public = PublicKey::from(&secret);
        let keypair = Keypair { secret, public };

        let miner_address = bs58::encode(keypair.public.to_bytes()).into_string();
        let height = 42u64;
        let block_reward = 500u64;
        let proof_rewards = vec![ProofRewardEntry {
            miner_pubkey: "miner1".to_string(),
            reward_amount: 100,
            proof_hash: "abc123".to_string(),
        }];

        let msg = hash_coinbase(height, &miner_address, block_reward, &proof_rewards);
        let signature = keypair.sign(&msg).to_bytes().to_vec();

        let cb = CoinbaseTransaction {
            tx_hash: msg,
            height,
            miner_address,
            block_reward: block_reward + 1,
            proof_rewards,
            signature,
        };
        let result = verify_coinbase_sig(&cb);
        assert_eq!(result, Err(TxError::InvalidSignature));
    }

    #[test]
    fn test_transaction_tx_hash() {
        let hash1 = [1u8; 32];
        let hash2 = [2u8; 32];
        let transfer_tx = TransferTransaction {
            tx_hash: hash1,
            nonce: 1,
            from: "from".to_string(),
            to: "to".to_string(),
            token: TokenType::TribeChain,
            amount: 100,
            fee: 1,
            signature: vec![],
            timestamp: 0,
        };
        let coinbase_tx = CoinbaseTransaction {
            tx_hash: hash2,
            height: 42,
            miner_address: "miner".to_string(),
            block_reward: 500,
            proof_rewards: vec![],
            signature: vec![],
        };

        assert_eq!(Transaction::Transfer(transfer_tx).tx_hash(), &hash1);
        assert_eq!(Transaction::Coinbase(coinbase_tx).tx_hash(), &hash2);
    }
}
