use std::collections::HashMap;
use std::sync::Arc;

use pot_o_core::TokenType;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tracing;

/// A single entry in the local token ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub address: String,
    pub token: TokenType,
    pub balance: u64,
}

/// Receipt returned by a successful transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxReceipt {
    pub tx_hash: String,
    pub from: String,
    pub to: String,
    pub token: TokenType,
    pub amount: u64,
    pub fee: u64,
    pub block_height: u64,
    pub timestamp: u64,
}

/// In-memory token ledger with JSON persistence.
pub struct Ledger {
    balances: HashMap<(String, TokenType), u64>,
    tx_history: Vec<TxReceipt>,
    block_height: u64,
    protocol_fee_address: String,
    modified: bool,
}

impl Ledger {
    pub fn new(protocol_fee_address: String) -> Self {
        Self {
            balances: HashMap::new(),
            tx_history: Vec::new(),
            block_height: 0,
            protocol_fee_address,
            modified: false,
        }
    }

    pub fn balance_of(&self, address: &str, token: &TokenType) -> u64 {
        self.balances
            .get(&(address.to_string(), token.clone()))
            .copied()
            .unwrap_or(0)
    }

    pub fn issue(&mut self, to: &str, token: &TokenType, amount: u64) {
        let key = (to.to_string(), token.clone());
        let entry = self.balances.entry(key).or_insert(0);
        *entry = entry.saturating_add(amount);
        self.block_height = self.block_height.saturating_add(1);
        self.modified = true;
    }

    pub fn transfer(
        &mut self,
        from: &str,
        to: &str,
        token: &TokenType,
        amount: u64,
        fee: u64,
    ) -> Result<TxReceipt, String> {
        if amount == 0 {
            return Err("Transfer amount must be positive".into());
        }
        let total = amount.checked_add(fee).ok_or("Overflow in amount + fee")?;
        let from_key = (from.to_string(), token.clone());
        let from_bal = self.balances.get(&from_key).copied().unwrap_or(0);
        if from_bal < total {
            return Err(format!(
                "Insufficient balance: have {}, need {} (amount {} + fee {})",
                from_bal, total, amount, fee
            ));
        }

        self.balances.insert(from_key, from_bal - total);

        let to_key = (to.to_string(), token.clone());
        let to_bal = self.balances.entry(to_key).or_insert(0);

        if fee > 0 && !self.protocol_fee_address.is_empty() && self.protocol_fee_address == to {
            *to_bal = to_bal.saturating_add(amount + fee);
        } else {
            *to_bal = to_bal.saturating_add(amount);
            if fee > 0 && !self.protocol_fee_address.is_empty() {
                let fee_key = (self.protocol_fee_address.clone(), token.clone());
                let fee_bal = self.balances.entry(fee_key).or_insert(0);
                *fee_bal = fee_bal.saturating_add(fee);
            }
        }

        self.block_height = self.block_height.saturating_add(1);
        self.modified = true;

        let mut hasher = Sha256::new();
        hasher.update(from.as_bytes());
        hasher.update(to.as_bytes());
        hasher.update(amount.to_le_bytes());
        hasher.update(fee.to_le_bytes());
        hasher.update(self.block_height.to_le_bytes());
        let tx_hash = hex::encode(hasher.finalize());

        let receipt = TxReceipt {
            tx_hash,
            from: from.to_string(),
            to: to.to_string(),
            token: token.clone(),
            amount,
            fee,
            block_height: self.block_height,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };
        self.tx_history.push(receipt.clone());

        Ok(receipt)
    }

    pub fn tx_history(&self) -> &[TxReceipt] {
        &self.tx_history
    }

    pub fn tx_history_for(&self, address: &str) -> Vec<TxReceipt> {
        self.tx_history
            .iter()
            .filter(|t| t.from == address || t.to == address)
            .cloned()
            .collect()
    }

    pub fn block_height(&self) -> u64 {
        self.block_height
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn clear_modified(&mut self) {
        self.modified = false;
    }

    pub fn protocol_fee_address(&self) -> &str {
        &self.protocol_fee_address
    }
}

/// Load ledger from a JSON file.
pub fn load_ledger(path: &str, protocol_fee_address: &str) -> Ledger {
    let content = std::fs::read_to_string(path).ok();
    let entries: Vec<LedgerEntry> = match content {
        Some(ref s) => serde_json::from_str(s).unwrap_or_default(),
        None => Vec::new(),
    };
    let mut ledger = Ledger::new(protocol_fee_address.to_string());
    for entry in entries {
        let key = (entry.address, entry.token);
        ledger.balances.insert(key, entry.balance);
    }
    ledger.modified = false;
    ledger
}

/// Persist ledger to a JSON file asynchronously.
pub fn spawn_persist_ledger(ledger: Arc<RwLock<Ledger>>, path: String) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            let entries: Vec<LedgerEntry> = {
                let l = ledger.read().await;
                if !l.is_modified() {
                    continue;
                }
                l.balances
                    .iter()
                    .map(|((addr, token), bal)| LedgerEntry {
                        address: addr.clone(),
                        token: token.clone(),
                        balance: *bal,
                    })
                    .collect()
            };
            if let Err(e) = std::fs::write(&path, serde_json::to_string_pretty(&entries).unwrap()) {
                tracing::warn!(error = %e, "Failed to persist ledger");
            } else {
                let mut l = ledger.write().await;
                l.clear_modified();
            }
        }
    });
}

/// Default path for the ledger JSON file.
pub const DEFAULT_LEDGER_PATH: &str = "ledger.json";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_and_balance() {
        let mut ledger = Ledger::new("protocol".into());
        ledger.issue("alice", &TokenType::TribeChain, 1000);
        assert_eq!(ledger.balance_of("alice", &TokenType::TribeChain), 1000);
        assert_eq!(ledger.balance_of("bob", &TokenType::TribeChain), 0);
        assert_eq!(ledger.block_height(), 1);
    }

    #[test]
    fn test_transfer_basic() {
        let mut ledger = Ledger::new("protocol".into());
        ledger.issue("alice", &TokenType::TribeChain, 1000);
        let receipt = ledger
            .transfer("alice", "bob", &TokenType::TribeChain, 300, 10)
            .unwrap();
        assert_eq!(receipt.amount, 300);
        assert_eq!(receipt.fee, 10);
        assert_eq!(ledger.balance_of("alice", &TokenType::TribeChain), 690);
        assert_eq!(ledger.balance_of("bob", &TokenType::TribeChain), 300);
        assert_eq!(ledger.balance_of("protocol", &TokenType::TribeChain), 10);
    }

    #[test]
    fn test_transfer_insufficient_balance() {
        let mut ledger = Ledger::new("protocol".into());
        ledger.issue("alice", &TokenType::TribeChain, 100);
        let err = ledger
            .transfer("alice", "bob", &TokenType::TribeChain, 200, 0)
            .unwrap_err();
        assert!(err.contains("Insufficient balance"));
    }

    #[test]
    fn test_transfer_zero_amount() {
        let mut ledger = Ledger::new("protocol".into());
        let err = ledger
            .transfer("alice", "bob", &TokenType::TribeChain, 0, 0)
            .unwrap_err();
        assert!(err.contains("positive"));
    }

    #[test]
    fn test_tx_history() {
        let mut ledger = Ledger::new("protocol".into());
        ledger.issue("alice", &TokenType::TribeChain, 1000);
        ledger
            .transfer("alice", "bob", &TokenType::TribeChain, 200, 5)
            .unwrap();
        assert_eq!(ledger.tx_history().len(), 1);
        let alice_txs = ledger.tx_history_for("alice");
        assert_eq!(alice_txs.len(), 1);
        let bob_txs = ledger.tx_history_for("bob");
        assert_eq!(bob_txs.len(), 1);
        let charlie_txs = ledger.tx_history_for("charlie");
        assert_eq!(charlie_txs.len(), 0);
    }

    #[test]
    fn test_load_ledger_persistence_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_ledger.json");
        let path_str = path.to_str().unwrap().to_string();

        {
            let mut ledger = Ledger::new("fee_addr".into());
            ledger.issue("alice", &TokenType::PTtC, 500);
            let entries: Vec<LedgerEntry> = ledger
                .balances
                .iter()
                .map(|((addr, token), bal)| LedgerEntry {
                    address: addr.clone(),
                    token: token.clone(),
                    balance: *bal,
                })
                .collect();
            std::fs::write(&path, serde_json::to_string_pretty(&entries).unwrap()).unwrap();
        }

        let loaded = load_ledger(&path_str, "fee_addr");
        assert_eq!(loaded.balance_of("alice", &TokenType::PTtC), 500);
        assert_eq!(loaded.balance_of("bob", &TokenType::PTtC), 0);
        assert!(!loaded.is_modified());
    }

    #[test]
    fn test_no_fee_when_protocol_is_recipient() {
        let mut ledger = Ledger::new("bob".into());
        ledger.issue("alice", &TokenType::TribeChain, 1000);
        ledger
            .transfer("alice", "bob", &TokenType::TribeChain, 300, 10)
            .unwrap();
        assert_eq!(ledger.balance_of("alice", &TokenType::TribeChain), 690);
        assert_eq!(ledger.balance_of("bob", &TokenType::TribeChain), 310);
    }

    #[test]
    fn test_multiple_token_types_independent() {
        let mut ledger = Ledger::new("protocol".into());
        ledger.issue("alice", &TokenType::TribeChain, 100);
        ledger.issue("alice", &TokenType::PTtC, 200);
        assert_eq!(ledger.balance_of("alice", &TokenType::TribeChain), 100);
        assert_eq!(ledger.balance_of("alice", &TokenType::PTtC), 200);
        ledger
            .transfer("alice", "bob", &TokenType::TribeChain, 50, 0)
            .unwrap();
        assert_eq!(ledger.balance_of("alice", &TokenType::TribeChain), 50);
        assert_eq!(ledger.balance_of("alice", &TokenType::PTtC), 200);
    }
}
