use ed25519_dalek::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TokenType {
    TribeChain,
    PTtC,
    NMTC,
    STOMP,
    AUM,
    AI3,
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::TribeChain => write!(f, "TRIBECHAIN"),
            TokenType::PTtC => write!(f, "PTTC"),
            TokenType::NMTC => write!(f, "NMTC"),
            TokenType::STOMP => write!(f, "STOMP"),
            TokenType::AUM => write!(f, "AUM"),
            TokenType::AI3 => write!(f, "AI3"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferTransaction {
    pub tx_hash: String,
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
    pub tx_hash: String,
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
    pub fn tx_hash(&self) -> &str {
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
    CoinbaseNotFirst,
    CoinbaseWrongToken,
    CoinbaseRewardMismatch,
    SupplyCapExceeded,
    CoinbaseImmature,
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
            TxError::CoinbaseNotFirst => write!(f, "coinbase transaction must be first in block"),
            TxError::CoinbaseWrongToken => {
                write!(f, "coinbase transaction must use TRIBECHAIN token")
            }
            TxError::CoinbaseRewardMismatch => {
                write!(f, "coinbase reward does not match expected block reward")
            }
            TxError::SupplyCapExceeded => write!(f, "supply cap exceeded"),
            TxError::CoinbaseImmature => write!(f, "coinbase reward is not yet spendable"),
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
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(from.as_bytes());
    hasher.update(nonce.to_le_bytes());
    hasher.update(to.as_bytes());
    hasher.update(token.to_string().as_bytes());
    hasher.update(amount.to_le_bytes());
    hasher.update(fee.to_le_bytes());
    hex::encode(hasher.finalize())
}

pub fn hash_coinbase(height: u64, miner_address: &str, block_reward: u64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(height.to_le_bytes());
    hasher.update(miner_address.as_bytes());
    hasher.update(block_reward.to_le_bytes());
    hex::encode(hasher.finalize())
}

pub fn verify_transfer_sig(
    from: &str,
    nonce: u64,
    to: &str,
    token: &TokenType,
    amount: u64,
    fee: u64,
    signature: &[u8],
) -> Result<(), TxError> {
    let pubkey_bytes = bs58::decode(from)
        .into_vec()
        .map_err(|_| TxError::InvalidSignature)?;
    let pubkey_array: [u8; 32] = pubkey_bytes
        .try_into()
        .map_err(|_| TxError::InvalidSignature)?;
    let public_key = PublicKey::from_bytes(&pubkey_array).map_err(|_| TxError::InvalidSignature)?;
    let sig = Signature::from_bytes(signature).map_err(|_| TxError::InvalidSignature)?;

    let mut message = Vec::new();
    message.extend_from_slice(&nonce.to_le_bytes());
    message.extend_from_slice(to.as_bytes());
    message.extend_from_slice(token.to_string().as_bytes());
    message.extend_from_slice(&amount.to_le_bytes());
    message.extend_from_slice(&fee.to_le_bytes());

    public_key
        .verify_strict(&message, &sig)
        .map_err(|_| TxError::InvalidSignature)
}

pub fn verify_coinbase_sig(
    miner_address: &str,
    height: u64,
    block_reward: u64,
    proof_rewards: &[ProofRewardEntry],
    signature: &[u8],
) -> Result<(), TxError> {
    let pubkey_bytes = bs58::decode(miner_address)
        .into_vec()
        .map_err(|_| TxError::InvalidSignature)?;
    let pubkey_array: [u8; 32] = pubkey_bytes
        .try_into()
        .map_err(|_| TxError::InvalidSignature)?;
    let public_key = PublicKey::from_bytes(&pubkey_array).map_err(|_| TxError::InvalidSignature)?;
    let sig = Signature::from_bytes(signature).map_err(|_| TxError::InvalidSignature)?;

    let mut message = Vec::new();
    message.extend_from_slice(&height.to_le_bytes());
    message.extend_from_slice(miner_address.as_bytes());
    message.extend_from_slice(&block_reward.to_le_bytes());
    for reward in proof_rewards {
        message.extend_from_slice(reward.proof_hash.as_bytes());
    }

    public_key
        .verify_strict(&message, &sig)
        .map_err(|_| TxError::InvalidSignature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Keypair, SecretKey, Signer};

    #[test]
    fn test_token_type_display() {
        assert_eq!(TokenType::TribeChain.to_string(), "TRIBECHAIN");
        assert_eq!(TokenType::PTtC.to_string(), "PTTC");
        assert_eq!(TokenType::NMTC.to_string(), "NMTC");
        assert_eq!(TokenType::STOMP.to_string(), "STOMP");
        assert_eq!(TokenType::AUM.to_string(), "AUM");
        assert_eq!(TokenType::AI3.to_string(), "AI3");
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
    }

    #[test]
    fn test_hash_transfer_deterministic() {
        let h1 = hash_transfer("Alice", 1, "Bob", &TokenType::TribeChain, 100, 1);
        let h2 = hash_transfer("Alice", 1, "Bob", &TokenType::TribeChain, 100, 1);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_hash_transfer_different_nonce() {
        let h1 = hash_transfer("Alice", 1, "Bob", &TokenType::TribeChain, 100, 1);
        let h2 = hash_transfer("Alice", 2, "Bob", &TokenType::TribeChain, 100, 1);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_coinbase_deterministic() {
        let h1 = hash_coinbase(42, "MinerPubkey", 500);
        let h2 = hash_coinbase(42, "MinerPubkey", 500);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_hash_coinbase_different_height() {
        let h1 = hash_coinbase(1, "Miner", 500);
        let h2 = hash_coinbase(2, "Miner", 500);
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

        let mut msg = Vec::new();
        msg.extend_from_slice(&nonce.to_le_bytes());
        msg.extend_from_slice(to.as_bytes());
        msg.extend_from_slice(token.to_string().as_bytes());
        msg.extend_from_slice(&amount.to_le_bytes());
        msg.extend_from_slice(&fee.to_le_bytes());

        let signature = keypair.sign(&msg).to_bytes().to_vec();
        let result = verify_transfer_sig(&from, nonce, &to, &token, amount, fee, &signature);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_transfer_sig_invalid() {
        let from = bs58::encode([0u8; 32]).into_string();
        let signature = vec![0u8; 64];
        let result =
            verify_transfer_sig(&from, 1, "to", &TokenType::TribeChain, 100, 1, &signature);
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

        let mut msg = Vec::new();
        msg.extend_from_slice(&nonce.to_le_bytes());
        msg.extend_from_slice(to.as_bytes());
        msg.extend_from_slice(token.to_string().as_bytes());
        msg.extend_from_slice(&amount.to_le_bytes());
        msg.extend_from_slice(&fee.to_le_bytes());

        let signature = keypair.sign(&msg).to_bytes().to_vec();
        let result = verify_transfer_sig(&from, nonce, &to, &token, amount + 1, fee, &signature);
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

        let mut msg = Vec::new();
        msg.extend_from_slice(&height.to_le_bytes());
        msg.extend_from_slice(miner_address.as_bytes());
        msg.extend_from_slice(&block_reward.to_le_bytes());
        for reward in &proof_rewards {
            msg.extend_from_slice(reward.proof_hash.as_bytes());
        }

        let signature = keypair.sign(&msg).to_bytes().to_vec();
        let result = verify_coinbase_sig(
            &miner_address,
            height,
            block_reward,
            &proof_rewards,
            &signature,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_coinbase_sig_invalid_pubkey() {
        let miner_address = bs58::encode([0u8; 32]).into_string();
        let signature = vec![0u8; 64];
        let result = verify_coinbase_sig(&miner_address, 42, 500, &[], &signature);
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

        let mut msg = Vec::new();
        msg.extend_from_slice(&height.to_le_bytes());
        msg.extend_from_slice(miner_address.as_bytes());
        msg.extend_from_slice(&block_reward.to_le_bytes());
        for reward in &proof_rewards {
            msg.extend_from_slice(reward.proof_hash.as_bytes());
        }

        let signature = keypair.sign(&msg).to_bytes().to_vec();
        let result = verify_coinbase_sig(
            &miner_address,
            height,
            block_reward + 1,
            &proof_rewards,
            &signature,
        );
        assert_eq!(result, Err(TxError::InvalidSignature));
    }

    #[test]
    fn test_transaction_tx_hash() {
        let transfer_tx = TransferTransaction {
            tx_hash: "abcdef".to_string(),
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
            tx_hash: "123456".to_string(),
            height: 42,
            miner_address: "miner".to_string(),
            block_reward: 500,
            proof_rewards: vec![],
            signature: vec![],
        };

        assert_eq!(Transaction::Transfer(transfer_tx).tx_hash(), "abcdef");
        assert_eq!(Transaction::Coinbase(coinbase_tx).tx_hash(), "123456");
    }
}
