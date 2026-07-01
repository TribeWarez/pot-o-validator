//! Application state and validator stats shared across HTTP handlers.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use hexchain_p2p::block::HexBlock;
use hexchain_p2p::hex_consensus::{HexChallenge, HexConsensus};
use pot_o_extensions::ledger::{block_reward_at_height, Ledger, TRIBE_HARD_CAP};
use pot_o_extensions::tx::{
    verify_coinbase_sig, verify_transfer_sig, CoinbaseTransaction, TokenType, TransferTransaction,
    TxError,
};
use pot_o_extensions::ExtensionRegistry;
use pot_o_mining::PotOConsensus;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use crate::config::ValidatorConfig;
use crate::device_registry::RegisteredDevice;

/// Aggregate statistics exposed by the validator API (e.g. /status).
#[derive(Debug, Clone, Default, Serialize)]
pub struct ValidatorStats {
    /// Total challenges issued since startup.
    pub total_challenges_issued: u64,
    /// Total proof submissions received.
    pub total_proofs_received: u64,
    /// Total proofs that passed verification.
    pub total_proofs_valid: u64,
    /// Count of active miners (from registry or on-chain).
    pub active_miners: u64,
    /// Uptime in seconds.
    pub uptime_secs: u64,
    /// Paths validated in the current challenge round (reset on new challenge).
    pub paths_in_block: u64,
    /// Tensor computations completed in the current challenge round (reset on new challenge).
    pub calcs_in_block: u64,
    /// Total TRIBE tokens minted through mining rewards.
    pub total_tribe_minted: u64,
    /// Total reward payouts (count of distribution events).
    pub total_rewards_paid: u64,
}

/// Shared state for the validator HTTP app (config, consensus, extensions, registry).
pub struct AppState {
    /// Loaded validator configuration.
    pub config: ValidatorConfig,
    /// PoT-O consensus and proof verification.
    pub consensus: PotOConsensus,
    /// Extension registry (chain, pool, network).
    pub extensions: ExtensionRegistry,
    /// Current active challenge (if any).
    pub current_challenge: RwLock<Option<pot_o_mining::Challenge>>,
    /// Aggregated stats for /status.
    pub stats: RwLock<ValidatorStats>,
    /// device_id (e.g. MAC) -> RegisteredDevice. Persisted so ESP mappings survive restarts.
    pub device_registry: RwLock<HashMap<String, RegisteredDevice>>,
    /// Path to the device registry JSON file.
    pub registry_path: String,

    /// hexchain 3D HCP lattice consensus engine.
    pub hex_consensus: HexConsensus,
    /// Current hexchain challenge (for miner discovery / status).
    pub hex_current_challenge: RwLock<Option<HexChallenge>>,

    /// TRIBE mint address (base58-encoded public key of the mint keypair).
    pub tribe_mint_address: String,
}

/// Builds the shared application state used by the Axum router.
pub fn create_app_state(
    cfg: ValidatorConfig,
    consensus: PotOConsensus,
    extensions: ExtensionRegistry,
    registry_path: String,
    device_registry: HashMap<String, RegisteredDevice>,
    hex_consensus: HexConsensus,
    tribe_mint_address: String,
) -> Arc<AppState> {
    Arc::new(AppState {
        config: cfg,
        consensus,
        extensions,
        current_challenge: RwLock::new(None),
        stats: RwLock::new(ValidatorStats::default()),
        device_registry: RwLock::new(device_registry),
        registry_path,
        hex_consensus,
        hex_current_challenge: RwLock::new(None),
        tribe_mint_address,
    })
}

/// Validate all transactions within a block against current ledger state.
pub fn validate_block_transactions(block: &HexBlock, ledger: &Ledger) -> Result<(), TxError> {
    let txs = block
        .transactions
        .as_ref()
        .ok_or(TxError::CoinbaseNotFirst)?;
    if txs.is_empty() {
        return Err(TxError::CoinbaseNotFirst);
    }

    // 1. Coinbase must be first
    let coinbase: CoinbaseTransaction =
        serde_json::from_value(txs[0].clone()).map_err(|_| TxError::CoinbaseNotFirst)?;

    // 2. Coinbase reward must match schedule
    let expected_reward = block_reward_at_height(block.height);
    if coinbase.block_reward != expected_reward {
        return Err(TxError::CoinbaseRewardMismatch);
    }

    // 3. Verify coinbase signature
    verify_coinbase_sig(&coinbase)?;

    // 4. Supply cap check
    let total_minted = coinbase.block_reward
        + coinbase
            .proof_rewards
            .iter()
            .map(|p| p.reward_amount)
            .sum::<u64>();
    let current_supply = ledger.total_supply_of(&TokenType::TribeChain);
    if current_supply
        .checked_add(total_minted)
        .map_or(true, |s| s > TRIBE_HARD_CAP)
    {
        return Err(TxError::SupplyCapExceeded);
    }

    // 5. Validate transfer transactions
    let mut seen_hashes = HashSet::new();
    let mut seen_from_nonces = HashMap::new();
    for tx_val in &txs[1..] {
        let tx: TransferTransaction =
            serde_json::from_value(tx_val.clone()).map_err(|_| TxError::InvalidSignature)?;

        // Amount check
        if tx.amount == 0 {
            return Err(TxError::AmountZero);
        }

        // Self-transfer check
        if tx.from == tx.to {
            return Err(TxError::SelfTransfer);
        }

        // Signature
        verify_transfer_sig(&tx)?;

        // Duplicate hash
        if !seen_hashes.insert(tx.tx_hash) {
            return Err(TxError::DuplicateTransaction);
        }

        // Conflicting nonces within block
        let entry = seen_from_nonces.entry(tx.from.clone()).or_insert(0u64);
        let expected = ledger.current_nonce(&tx.from) + *entry;
        if tx.nonce != expected {
            return Err(TxError::InvalidNonce);
        }
        *entry += 1;

        // Balance check (at parent state)
        let balance = ledger.balance_of(&tx.from, &tx.token);
        if balance < tx.amount + tx.fee {
            return Err(TxError::InsufficientBalance);
        }
    }

    // 6. Merkle root check
    let computed_root = compute_tx_merkle_root(txs);
    if block.tx_merkle_root != computed_root {
        return Err(TxError::MerkleRootMismatch);
    }

    Ok(())
}

/// Compute a merkle root over a list of serialized transactions.
fn compute_tx_merkle_root(txs: &[serde_json::Value]) -> [u8; 32] {
    let leaves: Vec<[u8; 32]> = txs
        .iter()
        .map(|tx| {
            let data = serde_json::to_string(tx).unwrap_or_else(|_| String::new());
            let mut hasher = Sha256::new();
            hasher.update(data.as_bytes());
            let result = hasher.finalize();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&result);
            hash
        })
        .collect();

    if leaves.is_empty() {
        return [0u8; 32];
    }
    let mut level = leaves;
    while level.len() > 1 {
        let mut next = Vec::new();
        for chunk in level.chunks(2) {
            let mut hasher = Sha256::new();
            hasher.update(&chunk[0]);
            hasher.update(if chunk.len() > 1 {
                &chunk[1]
            } else {
                &chunk[0]
            });
            let result = hasher.finalize();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&result);
            next.push(hash);
        }
        level = next;
    }
    level[0]
}
