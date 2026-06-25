//! TRIBE mining reward calculation and mint keypair management.

use solana_sdk::signature::{Keypair, Signer};
use std::path::Path;

/// BASE_REWARD: 50 TRIBE in micro-TRIBE units (50_000_000).
pub const BASE_REWARD: u64 = 50_000_000;

/// Calculate the mining reward for a valid proof.
///
/// Formula: `BASE_REWARD * difficulty / max(path_distance, 1)`
///
/// This scales reward with difficulty (harder challenges pay more) and
/// inversely with path distance (shorter paths demonstrate more efficient work).
pub fn calculate_mining_reward(difficulty: u64, path_distance: u32) -> u64 {
    let divisor = std::cmp::max(path_distance, 1) as u64;
    BASE_REWARD * difficulty / divisor
}

/// Load an existing TRIBE mint Ed25519 keypair from disk, or generate a new one.
///
/// The keypair is serialized as a JSON array of 64 bytes (secret || public).
/// Returns the base58-encoded public key as the TRIBE mint address.
pub fn load_or_create_tribe_mint(path: &str) -> String {
    let keypair = if Path::new(path).exists() {
        let content = std::fs::read_to_string(path).expect("Failed to read tribe mint keypair");
        let bytes: Vec<u8> =
            serde_json::from_str(&content).expect("Failed to parse tribe mint keypair");
        Keypair::from_bytes(&bytes).expect("Failed to deserialize tribe mint keypair")
    } else {
        let kp = Keypair::new();
        let bytes = kp.to_bytes().to_vec();
        std::fs::write(path, serde_json::to_string(&bytes).unwrap())
            .expect("Failed to save tribe mint keypair");
        tracing::info!(path = %path, "Generated new TRIBE mint keypair");
        kp
    };
    keypair.pubkey().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_reward_value() {
        assert_eq!(BASE_REWARD, 50_000_000);
    }

    #[test]
    fn test_calculate_mining_reward_basic() {
        // difficulty=2, path_distance=5: 50_000_000 * 2 / 5 = 20_000_000
        assert_eq!(calculate_mining_reward(2, 5), 20_000_000);
    }

    #[test]
    fn test_calculate_mining_reward_path_distance_zero() {
        // path_distance=0 is clamped to 1
        assert_eq!(calculate_mining_reward(1, 0), 50_000_000);
    }

    #[test]
    fn test_calculate_mining_reward_high_difficulty() {
        // difficulty=10, path_distance=1: 50_000_000 * 10 / 1 = 500_000_000
        assert_eq!(calculate_mining_reward(10, 1), 500_000_000);
    }

    #[test]
    fn test_calculate_mining_reward_high_path_distance() {
        // difficulty=1, path_distance=100: 50_000_000 * 1 / 100 = 500_000
        assert_eq!(calculate_mining_reward(1, 100), 500_000);
    }

    #[test]
    fn test_load_or_create_tribe_mint_creates_new() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tribe_mint_keypair.json");
        let path_str = path.to_str().unwrap().to_string();

        let address = load_or_create_tribe_mint(&path_str);
        assert!(!address.is_empty());
        assert!(path.exists());
    }

    #[test]
    fn test_load_or_create_tribe_mint_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tribe_mint_keypair.json");
        let path_str = path.to_str().unwrap().to_string();

        let address1 = load_or_create_tribe_mint(&path_str);
        let address2 = load_or_create_tribe_mint(&path_str);
        assert_eq!(address1, address2);
    }
}
