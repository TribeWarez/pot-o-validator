use crate::ledger::Ledger;
use crate::tx::{verify_transfer_sig, TransferTransaction, TxError};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::RwLock;
use tokio::sync::RwLock as AsyncRwLock;

/// Pending transaction pool
pub struct Mempool {
    txs: RwLock<HashMap<[u8; 32], TransferTransaction>>,
    address_nonces: RwLock<HashMap<String, u64>>,
    by_fee: RwLock<BTreeMap<(u64, u64), [u8; 32]>>, // (fee, timestamp_ns) → tx_hash
    max_size: usize,
    min_fee: u64,
}

impl Mempool {
    pub fn new(max_size: usize, min_fee: u64) -> Self {
        Mempool {
            txs: RwLock::new(HashMap::new()),
            address_nonces: RwLock::new(HashMap::new()),
            by_fee: RwLock::new(BTreeMap::new()),
            max_size,
            min_fee,
        }
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
    }

    pub fn pending(&self) -> Vec<TransferTransaction> {
        self.txs.read().unwrap().values().cloned().collect()
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
    use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer};
    use std::sync::Arc;

    async fn test_mempool_submit_and_drain_impl(ledger: &AsyncRwLock<Ledger>) {
        use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer};

        let secret = SecretKey::from_bytes(&[42u8; 32]).unwrap();
        let public = PublicKey::from(&secret);
        let keypair = Keypair { secret, public };
        let from = bs58::encode(keypair.public.to_bytes()).into_string();
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
        let signature = keypair.sign(&tx_hash).to_bytes().to_vec();

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
