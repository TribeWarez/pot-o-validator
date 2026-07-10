use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use hexchain_p2p::block::HexBlock;
use hexchain_p2p::hex_consensus::{HexChallenge, HexConsensus};
use hexchain_p2p::BlockStore;
use pot_o_extensions::ledger::{block_reward_at_height, Ledger, TRIBE_HARD_CAP};
use pot_o_extensions::tx::{
    verify_coinbase_sig, verify_transfer_sig, CoinbaseTransaction, TokenType, TransferTransaction,
    TxError,
};
use pot_o_extensions::ExtensionRegistry;
use pot_o_extensions::Mempool;
use pot_o_mining::PotOConsensus;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use crate::auth::AuthState;
use crate::config::ValidatorConfig;
use crate::device_registry::RegisteredDevice;

pub const RELAYED_BLOCKS_MAX: usize = 1000;

#[derive(Debug, Clone, Default, Serialize)]
pub struct ValidatorStats {
    pub total_challenges_issued: u64,
    pub total_proofs_received: u64,
    pub total_proofs_valid: u64,
    pub total_proofs_rejected: u64,
    pub active_miners: u64,
    pub uptime_secs: u64,
    pub paths_in_block: u64,
    pub calcs_in_block: u64,
    pub total_tribe_minted: u64,
    pub total_rewards_paid: u64,
}

pub struct AppState {
    pub config: ValidatorConfig,
    pub consensus: PotOConsensus,
    pub extensions: ExtensionRegistry,
    pub current_challenge: RwLock<Option<pot_o_mining::Challenge>>,
    pub stats: RwLock<ValidatorStats>,
    pub device_registry: RwLock<HashMap<String, RegisteredDevice>>,
    pub registry_path: String,
    pub hex_consensus: HexConsensus,
    pub hex_current_challenge: RwLock<Option<HexChallenge>>,
    pub auth: AuthState,
    pub canonical_tip_height: RwLock<u64>,
    /// Optional URL of the wallet service to record mining rewards
    pub wallet_url: Option<String>,
    /// Proof trace store for recent proof submissions
    pub proof_traces: pot_o_extensions::proof_trace::ProofTraceStore,
    pub relayed_blocks: RwLock<VecDeque<[u8; 32]>>,
}

pub fn create_app_state(
    cfg: ValidatorConfig,
    consensus: PotOConsensus,
    extensions: ExtensionRegistry,
    registry_path: String,
    device_registry: HashMap<String, RegisteredDevice>,
    hex_consensus: HexConsensus,
    wallet_url: Option<String>,
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
        auth: AuthState::new(),
        canonical_tip_height: RwLock::new(0),
        wallet_url,
        proof_traces: pot_o_extensions::proof_trace::ProofTraceStore::new(1000),
        relayed_blocks: RwLock::new(VecDeque::new()),
    })
}

pub fn validate_block_transactions(block: &HexBlock, ledger: &Ledger) -> Result<(), TxError> {
    let txs = block
        .transactions
        .as_ref()
        .ok_or(TxError::CoinbaseNotFirst)?;
    if txs.is_empty() {
        return Err(TxError::CoinbaseNotFirst);
    }

    let coinbase: CoinbaseTransaction =
        serde_json::from_value(txs[0].clone()).map_err(|_| TxError::CoinbaseNotFirst)?;

    let expected_reward = block_reward_at_height(block.height);
    if coinbase.block_reward != expected_reward {
        return Err(TxError::CoinbaseRewardMismatch);
    }

    verify_coinbase_sig(&coinbase)?;

    let total_minted = coinbase.block_reward
        + coinbase
            .proof_rewards
            .iter()
            .map(|p| p.reward_amount)
            .sum::<u64>();
    let current_supply = ledger.total_supply_of(&TokenType::TribeChain);
    if current_supply
        .checked_add(total_minted)
        .is_none_or(|s| s > TRIBE_HARD_CAP)
    {
        return Err(TxError::SupplyCapExceeded);
    }

    let mut seen_hashes = HashSet::new();
    let mut seen_from_nonces = HashMap::new();
    for tx_val in &txs[1..] {
        let tx: TransferTransaction =
            serde_json::from_value(tx_val.clone()).map_err(|_| TxError::InvalidSignature)?;

        if tx.amount == 0 {
            return Err(TxError::AmountZero);
        }

        if tx.from == tx.to {
            return Err(TxError::SelfTransfer);
        }

        verify_transfer_sig(&tx)?;

        if !seen_hashes.insert(tx.tx_hash) {
            return Err(TxError::DuplicateTransaction);
        }

        let entry = seen_from_nonces.entry(tx.from.clone()).or_insert(0u64);
        let expected = ledger.current_nonce(&tx.from) + *entry;
        if tx.nonce != expected {
            return Err(TxError::InvalidNonce);
        }
        *entry += 1;

        let balance = ledger.balance_of(&tx.from, &tx.token);
        if balance < tx.amount + tx.fee {
            return Err(TxError::InsufficientBalance);
        }
    }

    let computed_root = compute_tx_merkle_root(txs);
    if block.tx_merkle_root != computed_root {
        return Err(TxError::MerkleRootMismatch);
    }

    Ok(())
}

pub fn rollback_ledger_to(
    ledger: &mut Ledger,
    block_store: &BlockStore,
    target_height: u64,
    canonical_tip_height: &mut u64,
) -> Result<(), String> {
    while ledger.block_height() > target_height {
        let current_height = ledger.block_height();
        let stored = block_store.at_height(current_height).ok_or_else(|| {
            format!(
                "Block at height {} not found in block store",
                current_height
            )
        })?;
        let block: HexBlock = serde_json::from_str(&stored.block_json)
            .map_err(|e| format!("Failed to deserialize block: {}", e))?;
        ledger.rollback_block(&block)?;
        *canonical_tip_height = current_height - 1;
    }
    Ok(())
}

pub fn accept_block(
    block: &HexBlock,
    ledger: &mut Ledger,
    mempool: Option<&Mempool>,
    block_store: Option<&BlockStore>,
) -> Result<Vec<pot_o_extensions::TxReceipt>, String> {
    validate_block_transactions(block, ledger)
        .map_err(|e| format!("Transaction validation failed: {:?}", e))?;

    let receipts = ledger.apply_block(block)?;

    if let Some(mp) = mempool {
        if let Some(txs) = &block.transactions {
            let mut remove_hashes = Vec::new();
            for tx_val in &txs[1..] {
                if let Ok(tx) = serde_json::from_value::<pot_o_extensions::tx::TransferTransaction>(
                    tx_val.clone(),
                ) {
                    remove_hashes.push(tx.tx_hash);
                }
            }
            mp.remove(&remove_hashes);
        }
    }

    if let Some(bs) = block_store {
        bs.append(block);
    }

    Ok(receipts)
}

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
            hasher.update(chunk[0]);
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
