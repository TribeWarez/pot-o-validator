use crate::ledger::Ledger;
use crate::tx::{verify_transfer_sig, TransferTransaction, TxError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::RwLock;
use tokio::sync::RwLock as AsyncRwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MempoolEntry {
    tx: TransferTransaction,
    pending_nonces: HashMap<String, u64>,
}

/// Pending transaction pool
pub struct Mempool {
    txs: RwLock<HashMap<[u8; 32], TransferTransaction>>,
    address_nonces: RwLock<HashMap<String, u64>>,
    by_fee: RwLock<BTreeMap<(u64, u64), [u8; 32]>>, // (fee, timestamp_ns) → tx_hash
    max_size: usize,
    min_fee: u64,
    path: RwLock<String>,
    modified: RwLock<bool>,
}

impl Mempool {
    pub fn new(max_size: usize, min_fee: u64) -> Self {
        Mempool {
            txs: RwLock::new(HashMap::new()),
            address_nonces: RwLock::new(HashMap::new()),
            by_fee: RwLock::new(BTreeMap::new()),
            max_size,
            min_fee,
            path: RwLock::new(String::new()),
            modified: RwLock::new(false),
        }
    }

    pub fn set_path(&self, path: &str) {
        *self.path.write().unwrap() = path.to_string();
    }

    pub fn load_from_file(&self, path: &str) {
        let content = std::fs::read_to_string(path).ok();
        if let Some(ref s) = content {
            if let Ok(entries) = serde_json::from_str::<Vec<MempoolEntry>>(s) {
                let mut txs = self.txs.write().unwrap();
                let mut by_fee = self.by_fee.write().unwrap();
                let mut nonces = self.address_nonces.write().unwrap();
                let base_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64;
                for (i, entry) in entries.into_iter().enumerate() {
                    let hash = entry.tx.tx_hash;
                    txs.insert(hash, entry.tx);
                    by_fee.insert((0, base_time + i as u64), hash);
                    for (addr, n) in entry.pending_nonces {
                        nonces.insert(addr, n);
                    }
                }
            }
        }
        *self.path.write().unwrap() = path.to_string();
    }

    pub fn save_to_file(&self) -> Result<(), String> {
        let (path, entries) = {
            let path = self.path.read().unwrap().clone();
            if path.is_empty() {
                return Ok(());
            }
            let txs = self.txs.read().unwrap();
            let nonces = self.address_nonces.read().unwrap();
            let entries: Vec<MempoolEntry> = txs
                .values()
                .map(|tx| MempoolEntry {
                    tx: tx.clone(),
                    pending_nonces: nonces.clone(),
                })
                .collect();
            (path, entries)
        };
        let json = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
        let tmp_path = format!("{}.tmp", path);
        std::fs::write(&tmp_path, &json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())?;
        *self.modified.write().unwrap() = false;
        Ok(())
    }

    pub fn is_modified(&self) -> bool {
        *self.modified.read().unwrap()
    }

    /// Submit a transaction to the mempool after validation.
    /// Uses `RwLock::read()` for non-blocking reads since this is called from async context.
    pub async fn submit(
        &self,
        tx: TransferTransaction,
        ledger: &AsyncRwLock<Ledger>,
    ) -> Result<[u8; 32], TxError> {
        // 1. Fee check
        if tx.fee < self.min_fee {
            return Err(TxError::FeeTooLow);
        }

        // 2. Amount check
        if tx.amount == 0 {
            return Err(TxError::AmountZero);
        }

        // 3. Self-transfer check
        if tx.from == tx.to {
            return Err(TxError::SelfTransfer);
        }

        // 4. Signature verification
        verify_transfer_sig(&tx)?;

        // 5. Duplicate check
        if self.txs.read().unwrap().contains_key(&tx.tx_hash) {
            return Err(TxError::DuplicateTransaction);
        }

        // 6. Nonce and balance check against ledger + pending
        // Scope ledger read lock so it drops before acquiring mempool write locks (avoid ABBA deadlock)
        let (ledger_nonce, balance) = {
            let guard = ledger.read().await;
            (
                guard.current_nonce(&tx.from),
                guard.balance_of(&tx.from, &tx.token),
            )
        };
        let pending_count = self
            .address_nonces
            .read()
            .unwrap()
            .get(&tx.from)
            .copied()
            .unwrap_or(0);
        let expected_nonce = ledger_nonce + pending_count;
        if tx.nonce != expected_nonce {
            return Err(TxError::InvalidNonce);
        }

        let pending_amount: u64 = self
            .txs
            .read()
            .unwrap()
            .values()
            .filter(|t| t.from == tx.from && t.token == tx.token)
            .map(|t| t.amount + t.fee)
            .sum();
        if balance < tx.amount + tx.fee + pending_amount {
            return Err(TxError::InsufficientBalance);
        }

        // 7. Insert (ledger lock dropped, safe to acquire mempool locks)
        let tx_hash = tx.tx_hash;
        let fee = tx.fee;
        let from = tx.from.clone();
        self.txs.write().unwrap().insert(tx_hash, tx);
        *self
            .address_nonces
            .write()
            .unwrap()
            .entry(from)
            .or_insert(0) += 1;

        // Use current time nanos as tiebreaker for same-fee ordering
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        self.by_fee.write().unwrap().insert((fee, ts), tx_hash);

        // Evict if over capacity
        self.evict_over_capacity();

        *self.modified.write().unwrap() = true;

        Ok(tx_hash)
    }

    /// Drain highest-fee transactions for block inclusion
    pub fn drain_for_block(&self, max_count: usize, min_fee: u64) -> Vec<TransferTransaction> {
        let mut result = Vec::new();
        let mut to_remove = Vec::new();

        let by_fee = self.by_fee.read().unwrap();
        for ((fee, ts), hash) in by_fee.iter().rev() {
            if *fee < min_fee {
                break;
            }
            if result.len() >= max_count {
                break;
            }
            if let Some(tx) = self.txs.read().unwrap().get(hash) {
                result.push(tx.clone());
                to_remove.push((*fee, *ts, *hash));
            }
        }
        drop(by_fee);

        // Remove drained txs (single retain pass using HashSet)
        let mut txs = self.txs.write().unwrap();
        let mut by_fee_map = self.by_fee.write().unwrap();
        let mut nonces = self.address_nonces.write().unwrap();
        let remove_set: HashSet<[u8; 32]> = to_remove.iter().map(|(_, _, h)| *h).collect();
        for hash in &remove_set {
            if let Some(tx) = txs.remove(hash) {
                let addr_count = nonces.get(&tx.from).copied().unwrap_or(1).saturating_sub(1);
                if addr_count == 0 {
                    nonces.remove(&tx.from);
                } else {
                    nonces.insert(tx.from, addr_count);
                }
            }
        }
        by_fee_map.retain(|_k, v| !remove_set.contains(v));

        *self.modified.write().unwrap() = true;
        result
    }

    /// Remove confirmed transactions (after block acceptance)
    pub fn remove(&self, tx_hashes: &[[u8; 32]]) {
        let mut txs = self.txs.write().unwrap();
        let mut by_fee_map = self.by_fee.write().unwrap();
        let mut nonces = self.address_nonces.write().unwrap();

        let remove_set: HashSet<[u8; 32]> = tx_hashes.iter().copied().collect();
        for hash in &remove_set {
            if let Some(tx) = txs.remove(hash) {
                let addr_count = nonces.get(&tx.from).copied().unwrap_or(1).saturating_sub(1);
                if addr_count == 0 {
                    nonces.remove(&tx.from);
                } else {
                    nonces.insert(tx.from, addr_count);
                }
            }
        }
        by_fee_map.retain(|_k, v| !remove_set.contains(v));
        *self.modified.write().unwrap() = true;
    }

    /// Revalidate all pending txs against current ledger state
    pub fn revalidate(&self, ledger: &AsyncRwLock<Ledger>) {
        let (to_remove, _balance_map) = {
            let ledger = ledger.blocking_read();
            let mut to_remove = Vec::new();
            let mut balance_map = HashMap::new();
            let txs = self.txs.read().unwrap();
            for (hash, tx) in txs.iter() {
                let ledger_nonce = ledger.current_nonce(&tx.from);
                if tx.nonce < ledger_nonce {
                    to_remove.push(*hash);
                    continue;
                }
                let balance = ledger.balance_of(&tx.from, &tx.token);
                balance_map.insert(tx.from.clone(), balance);
                if balance < tx.amount + tx.fee {
                    to_remove.push(*hash);
                }
            }
            drop(txs);
            (to_remove, balance_map)
        };

        let mut txs = self.txs.write().unwrap();
        let mut by_fee_map = self.by_fee.write().unwrap();
        let mut nonces = self.address_nonces.write().unwrap();

        let remove_set: HashSet<[u8; 32]> = to_remove.iter().copied().collect();
        for hash in &remove_set {
            if let Some(tx) = txs.remove(hash) {
                let addr_count = nonces.get(&tx.from).copied().unwrap_or(1).saturating_sub(1);
                if addr_count == 0 {
                    nonces.remove(&tx.from);
                } else {
                    nonces.insert(tx.from, addr_count);
                }
            }
        }
        by_fee_map.retain(|_k, v| !remove_set.contains(v));
        *self.modified.write().unwrap() = true;
    }

    pub fn pending(&self) -> Vec<TransferTransaction> {
        self.txs.read().unwrap().values().cloned().collect()
    }

    pub fn hashes(&self) -> Vec<[u8; 32]> {
        self.txs.read().unwrap().keys().copied().collect()
    }

    pub fn get_tx(&self, hash: &[u8; 32]) -> Option<TransferTransaction> {
        self.txs.read().unwrap().get(hash).cloned()
    }

    pub fn len(&self) -> usize {
        self.txs.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn evict_over_capacity(&self) {
        let count = self.txs.read().unwrap().len();
        if count <= self.max_size {
            return;
        }
        let excess = count - self.max_size;
        let lowest_fee: Vec<[u8; 32]> = self
            .by_fee
            .read()
            .unwrap()
            .iter()
            .take(excess)
            .map(|(_, hash)| *hash)
            .collect();
        self.remove(&lowest_fee);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::Ledger;
    use crate::tx::{hash_transfer, TokenType};

    async fn test_mempool_submit_and_drain_impl(ledger: &AsyncRwLock<Ledger>) {
        use ed25519_dalek::{Signer, SigningKey};

        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        let from = bs58::encode(signing_key.verifying_key().to_bytes()).into_string();
        let to = bs58::encode([99u8; 32]).into_string();

        ledger
            .write()
            .await
            .issue(&from, &TokenType::TribeChain, 1000);
        let mempool = Mempool::new(1000, 0);

        let nonce = 0u64;
        let amount = 100u64;
        let fee = 1u64;
        let timestamp = 0u64;

        let tx_hash = hash_transfer(
            &from,
            nonce,
            &to,
            &TokenType::TribeChain,
            amount,
            fee,
            timestamp,
        );
        let signature = signing_key.sign(&tx_hash).to_bytes().to_vec();

        let tx = TransferTransaction {
            tx_hash,
            nonce,
            from: from.clone(),
            to,
            token: TokenType::TribeChain,
            amount,
            fee,
            signature,
            timestamp,
        };

        let result = mempool.submit(tx, ledger).await;
        assert!(result.is_ok(), "submit should succeed: {:?}", result);
        assert_eq!(mempool.len(), 1);

        let drained = mempool.drain_for_block(100, 0);
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].from, from);
        assert_eq!(mempool.len(), 0);
    }

    fn test_mempool_revalidate_impl(ledger: &AsyncRwLock<Ledger>) {
        let mempool = Mempool::new(1000, 0);
        assert_eq!(mempool.len(), 0);
        mempool.revalidate(ledger);
        assert_eq!(mempool.len(), 0);
    }

    #[tokio::test]
    async fn test_mempool_submit_and_drain() {
        let ledger = AsyncRwLock::new(Ledger::new("protocol".to_string()));
        test_mempool_submit_and_drain_impl(&ledger).await;
    }

    #[test]
    fn test_mempool_revalidate() {
        let ledger = AsyncRwLock::new(Ledger::new("protocol".to_string()));
        test_mempool_revalidate_impl(&ledger);
    }
}
