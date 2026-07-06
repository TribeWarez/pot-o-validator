use std::collections::HashMap;
use std::sync::Arc;

use pot_o_core::TokenType;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tracing;

use crate::tx::{CoinbaseTransaction, TransferTransaction};
use hexchain_p2p::block::HexBlock;

pub const TRIBE_HARD_CAP: u64 = 21_000_000_000_000_000;
pub const TRIBE_BLOCK_REWARD_INITIAL: u64 = 100_000_000_000;
pub const TRIBE_HALVING_INTERVAL: u64 = 210_000;
pub const TRIBE_PROOF_REWARD: u64 = 1_000_000_000;
pub const COINBASE_MATURITY_DEPTH: u64 = 100;

pub fn block_reward_at_height(height: u64) -> u64 {
    let halvings = height / TRIBE_HALVING_INTERVAL;
    if halvings >= 64 {
        return 0;
    }
    TRIBE_BLOCK_REWARD_INITIAL >> halvings
}

#[derive(Debug, Clone)]
pub struct CoinbaseEntry {
    pub amount: u64,
    pub mature_at_height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub address: String,
    pub token: TokenType,
    pub balance: u64,
}

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

pub struct Ledger {
    balances: HashMap<(String, TokenType), u64>,
    tx_history: Vec<TxReceipt>,
    block_height: u64,
    protocol_fee_address: String,
    modified: bool,
    nonces: HashMap<String, u64>,
    coinbase_maturity: HashMap<String, Vec<CoinbaseEntry>>,
    total_supply_map: HashMap<TokenType, u64>,
    last_interaction: HashMap<(String, TokenType), u64>, // block height of last interaction
}

impl Ledger {
    pub fn new(protocol_fee_address: String) -> Self {
        Self {
            balances: HashMap::new(),
            tx_history: Vec::new(),
            block_height: 0,
            protocol_fee_address,
            modified: false,
            nonces: HashMap::new(),
            coinbase_maturity: HashMap::new(),
            total_supply_map: HashMap::new(),
            last_interaction: HashMap::new(),
        }
    }

    pub fn balance_of(&self, address: &str, token: &TokenType) -> u64 {
        self.balances
            .get(&(address.to_string(), token.clone()))
            .copied()
            .unwrap_or(0)
    }

    pub fn balances(&self) -> &std::collections::HashMap<(String, TokenType), u64> {
        &self.balances
    }

    pub fn issue(&mut self, to: &str, token: &TokenType, amount: u64) {
        let key = (to.to_string(), token.clone());
        let entry = self.balances.entry(key).or_insert(0);
        *entry = entry.saturating_add(amount);
        
        // Update total supply map
        let total = self.total_supply_map.entry(token.clone()).or_insert(0);
        *total = total.saturating_add(amount);
        
        self.block_height = self.block_height.saturating_add(1);
        self.modified = true;
    }

    /// Try to issue tokens, respecting supply caps from token_config
    pub fn try_issue(&mut self, to: &str, token: &TokenType, amount: u64) -> Result<(), String> {
        use pot_o_core::token_config::token_config;
        
        // Get token configuration
        let config_set = token_config();
        
        // Check if token has a supply cap configured
        if let Some(config) = config_set.get(token) {
            // Calculate current total supply
            let current_supply = self.total_supply_of(token);
            
            // Check if adding this amount would exceed the cap
            if current_supply.saturating_add(amount) > config.supply_cap {
                return Err(format!(
                    "Supply cap exceeded for {:?}: current={}, requested={}, cap={}",
                    token, current_supply, amount, config.supply_cap
                ));
            }
        }
        
        // If all checks pass, issue the tokens
        let key = (to.to_string(), token.clone());
        let entry = self.balances.entry(key).or_insert(0);
        *entry = entry.saturating_add(amount);
        
        // Update total supply map
        let total = self.total_supply_map.entry(token.clone()).or_insert(0);
        *total = total.saturating_add(amount);
        
        self.block_height = self.block_height.saturating_add(1);
        self.modified = true;
        
        Ok(())
    }

    pub fn transfer(
        &mut self,
        from: &str,
        to: &str,
        token: &TokenType,
        amount: u64,
        fee: u64,
    ) -> Result<TxReceipt, String> {
        use pot_o_core::token_config::token_config;
        
        if amount == 0 {
            return Err("Transfer amount must be positive".into());
        }
        
        // Calculate burn amount based on token configuration
        let config_set = token_config();
        let burn = if let Some(config) = config_set.get(token) {
            config.calculate_burn(amount)
        } else {
            0
        };
        
        let total = amount.checked_add(fee).ok_or("Overflow in amount + fee")?;
        let from_key = (from.to_string(), token.clone());
        let from_bal = self.balances.get(&from_key).copied().unwrap_or(0);
        
        // Sender must have: amount + fee + burn
        let total_needed = total.checked_add(burn).ok_or("Overflow in total + burn")?;
        if from_bal < total_needed {
            return Err(format!(
                "Insufficient balance: have {}, need {} (amount {} + fee {} + burn {})",
                from_bal, total_needed, amount, fee, burn
            ));
        }

        self.balances.insert(from_key.clone(), from_bal - total_needed);

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

        // Update total supply to reflect burn
        if burn > 0 {
            let total = self.total_supply_map.entry(token.clone()).or_insert(0);
            *total = total.saturating_sub(burn);
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

    pub fn total_supply(&self, token: &TokenType) -> u64 {
        self.balances
            .iter()
            .filter(|((_, t), _)| t == token)
            .map(|(_, bal)| *bal)
            .sum()
    }

    pub fn current_nonce(&self, address: &str) -> u64 {
        self.nonces.get(address).copied().unwrap_or(0)
    }

    pub fn total_supply_of(&self, token: &TokenType) -> u64 {
        self.total_supply_map.get(token).copied().unwrap_or(0)
    }

    /// Record an interaction (transfer/issue) for an address-token pair
    pub fn update_interaction(&mut self, address: &str, token: &TokenType) {
        let key = (address.to_string(), token.clone());
        self.last_interaction.insert(key, self.block_height);
    }

    /// Get the block height of the last interaction for an address-token pair
    pub fn last_interaction(&self, address: &str, token: &TokenType) -> Option<u64> {
        self.last_interaction
            .get(&(address.to_string(), token.clone()))
            .copied()
    }

    /// Apply age-based decay to a balance based on blocks elapsed
    /// Returns the decayed amount using exponential decay formula
    pub fn apply_decay(&self, address: &str, token: &TokenType, blocks_elapsed: u64) -> u64 {
        use pot_o_core::token_config::token_config;
        
        let balance = self.balance_of(address, token);
        
        // Get token configuration
        let config_set = token_config();
        
        if let Some(config) = config_set.get(token) {
            config.apply_decay(balance, blocks_elapsed)
        } else {
            // No decay configured for this token
            balance
        }
    }

    pub fn is_coinbase_mature(&self, _address: &str, height: u64, current_height: u64) -> bool {
        current_height >= height + COINBASE_MATURITY_DEPTH
    }

    pub fn apply_transfer(
        &mut self,
        tx: &TransferTransaction,
        miner: &str,
    ) -> Result<TxReceipt, String> {
        if tx.amount == 0 {
            return Err("Transfer amount must be positive".into());
        }
        let total = tx
            .amount
            .checked_add(tx.fee)
            .ok_or("Overflow in amount + fee")?;
        let from_key = (tx.from.clone(), tx.token.clone());
        let from_bal = self.balances.get(&from_key).copied().unwrap_or(0);
        if from_bal < total {
            return Err(format!(
                "Insufficient balance: have {}, need {} (amount {} + fee {})",
                from_bal, total, tx.amount, tx.fee
            ));
        }

        self.balances.insert(from_key, from_bal - total);

        let to_key = (tx.to.clone(), tx.token.clone());
        let to_bal = self.balances.entry(to_key).or_insert(0);
        *to_bal = to_bal.saturating_add(tx.amount);

        if tx.fee > 0 {
            let miner_key = (miner.to_string(), tx.token.clone());
            let miner_bal = self.balances.entry(miner_key).or_insert(0);
            *miner_bal = miner_bal.saturating_add(tx.fee);
        }

        self.nonces
            .entry(tx.from.clone())
            .and_modify(|n| *n += 1)
            .or_insert(1);

        self.modified = true;

        let receipt = TxReceipt {
            tx_hash: hex::encode(tx.tx_hash),
            from: tx.from.clone(),
            to: tx.to.clone(),
            token: tx.token.clone(),
            amount: tx.amount,
            fee: tx.fee,
            block_height: self.block_height,
            timestamp: tx.timestamp,
        };
        self.tx_history.push(receipt.clone());

        Ok(receipt)
    }

    pub fn rollback_transfer(
        &mut self,
        tx: &TransferTransaction,
        miner: &str,
    ) -> Result<(), String> {
        let to_key = (tx.to.clone(), tx.token.clone());
        let to_bal = self.balances.get(&to_key).copied().unwrap_or(0);
        if to_bal < tx.amount {
            return Err(format!(
                "Insufficient balance to rollback recipient: have {}, need {}",
                to_bal, tx.amount
            ));
        }
        self.balances.insert(to_key, to_bal - tx.amount);

        if tx.fee > 0 {
            let miner_key = (miner.to_string(), tx.token.clone());
            let miner_bal = self.balances.get(&miner_key).copied().unwrap_or(0);
            if miner_bal < tx.fee {
                return Err(format!(
                    "Insufficient balance to rollback miner fee: have {}, need {}",
                    miner_bal, tx.fee
                ));
            }
            self.balances.insert(miner_key, miner_bal - tx.fee);
        }

        let from_key = (tx.from.clone(), tx.token.clone());
        let from_bal = self.balances.entry(from_key).or_insert(0);
        *from_bal = from_bal.saturating_add(tx.amount + tx.fee);

        self.nonces.entry(tx.from.clone()).and_modify(|n| {
            if *n > 0 {
                *n -= 1
            }
        });

        self.modified = true;
        Ok(())
    }

    pub fn apply_coinbase(&mut self, cb: &CoinbaseTransaction) -> Result<(), String> {
        let total_proof_rewards: u64 = cb.proof_rewards.iter().map(|pr| pr.reward_amount).sum();
        let total_mint = cb.block_reward + total_proof_rewards;

        let current_supply = self.total_supply_of(&TokenType::TribeChain);
        if current_supply + total_mint > TRIBE_HARD_CAP {
            return Err("Supply cap exceeded".into());
        }

        let miner_key = (cb.miner_address.clone(), TokenType::TribeChain);
        let miner_bal = self.balances.entry(miner_key).or_insert(0);
        *miner_bal = miner_bal.saturating_add(cb.block_reward);

        for pr in &cb.proof_rewards {
            let pr_key = (pr.miner_pubkey.clone(), TokenType::TribeChain);
            let pr_bal = self.balances.entry(pr_key).or_insert(0);
            *pr_bal = pr_bal.saturating_add(pr.reward_amount);
        }

        self.coinbase_maturity
            .entry(cb.miner_address.clone())
            .or_default()
            .push(CoinbaseEntry {
                amount: cb.block_reward,
                mature_at_height: cb.height + COINBASE_MATURITY_DEPTH,
            });

        *self
            .total_supply_map
            .entry(TokenType::TribeChain)
            .or_insert(0) += total_mint;

        self.modified = true;
        Ok(())
    }

    pub fn rollback_coinbase(&mut self, cb: &CoinbaseTransaction) -> Result<(), String> {
        let miner_key = (cb.miner_address.clone(), TokenType::TribeChain);
        let miner_bal = self.balances.entry(miner_key).or_insert(0);
        *miner_bal = miner_bal.saturating_sub(cb.block_reward);

        for pr in &cb.proof_rewards {
            let pr_key = (pr.miner_pubkey.clone(), TokenType::TribeChain);
            let pr_bal = self.balances.entry(pr_key).or_insert(0);
            *pr_bal = pr_bal.saturating_sub(pr.reward_amount);
        }

        if let Some(entries) = self.coinbase_maturity.get_mut(&cb.miner_address) {
            entries.pop();
        }

        let total_proof_rewards: u64 = cb.proof_rewards.iter().map(|pr| pr.reward_amount).sum();
        *self
            .total_supply_map
            .entry(TokenType::TribeChain)
            .or_insert(0) -= cb.block_reward + total_proof_rewards;

        self.modified = true;
        Ok(())
    }

    pub fn apply_block(&mut self, block: &HexBlock) -> Result<Vec<TxReceipt>, String> {
        let txs = block
            .transactions
            .as_ref()
            .ok_or_else(|| "Block has no transactions".to_string())?;

        if txs.is_empty() {
            return Err("Block has no transactions".into());
        }

        self.block_height = block.height;

        let cb: CoinbaseTransaction = serde_json::from_value(txs[0].clone())
            .map_err(|e| format!("Failed to deserialize coinbase: {}", e))?;
        let miner = cb.miner_address.clone();

        self.apply_coinbase(&cb)?;

        let mut receipts = Vec::new();
        for tx_val in txs[1..].iter() {
            let tx: TransferTransaction = serde_json::from_value(tx_val.clone())
                .map_err(|e| format!("Failed to deserialize transfer: {}", e))?;
            let receipt = self.apply_transfer(&tx, &miner)?;
            receipts.push(receipt);
        }

        Ok(receipts)
    }

    pub fn rollback_block(&mut self, block: &HexBlock) -> Result<(), String> {
        let txs = block
            .transactions
            .as_ref()
            .ok_or_else(|| "Block has no transactions".to_string())?;

        if txs.is_empty() {
            return Err("Block has no transactions".into());
        }

        let cb: CoinbaseTransaction = serde_json::from_value(txs[0].clone())
            .map_err(|e| format!("Failed to deserialize coinbase: {}", e))?;
        let miner = cb.miner_address.clone();

        for tx_val in txs[1..].iter().rev() {
            let tx: TransferTransaction = serde_json::from_value(tx_val.clone())
                .map_err(|e| format!("Failed to deserialize transfer: {}", e))?;
            self.rollback_transfer(&tx, &miner)?;
        }

        self.rollback_coinbase(&cb)?;

        Ok(())
    }

    pub fn mint_tokens(&mut self, to: &str, token: &TokenType, amount: u64) -> Result<(), String> {
        let current = self.total_supply_of(token);
        if current + amount > TRIBE_HARD_CAP && *token == TokenType::TribeChain {
            return Err("Supply cap exceeded".into());
        }
        self.issue(to, token, amount);
        *self.total_supply_map.entry(token.clone()).or_insert(0) += amount;
        Ok(())
    }
}

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

pub const DEFAULT_LEDGER_PATH: &str = "ledger.json";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tx::ProofRewardEntry;

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

    #[test]
    fn test_block_reward_halving() {
        assert_eq!(block_reward_at_height(0), TRIBE_BLOCK_REWARD_INITIAL);
        assert_eq!(
            block_reward_at_height(TRIBE_HALVING_INTERVAL),
            TRIBE_BLOCK_REWARD_INITIAL / 2
        );
        assert_eq!(block_reward_at_height(TRIBE_HALVING_INTERVAL * 64), 0);
    }

    #[test]
    fn test_apply_transfer_normal() {
        let mut ledger = Ledger::new("protocol".to_string());
        ledger
            .mint_tokens("alice", &TokenType::TribeChain, 1000)
            .unwrap();
        let tx = TransferTransaction {
            tx_hash: [0u8; 32],
            nonce: 0,
            from: "alice".to_string(),
            to: "bob".to_string(),
            token: TokenType::TribeChain,
            amount: 100,
            fee: 1,
            signature: vec![],
            timestamp: 0,
        };
        let result = ledger.apply_transfer(&tx, "miner").unwrap();
        assert_eq!(ledger.balance_of("alice", &TokenType::TribeChain), 899);
        assert_eq!(ledger.balance_of("bob", &TokenType::TribeChain), 100);
        assert_eq!(ledger.balance_of("miner", &TokenType::TribeChain), 1);
        assert_eq!(ledger.current_nonce("alice"), 1);
    }

    #[test]
    fn test_apply_transfer_insufficient() {
        let mut ledger = Ledger::new("protocol".to_string());
        ledger
            .mint_tokens("alice", &TokenType::TribeChain, 50)
            .unwrap();
        let tx = TransferTransaction {
            tx_hash: [0u8; 32],
            nonce: 0,
            from: "alice".to_string(),
            to: "bob".to_string(),
            token: TokenType::TribeChain,
            amount: 100,
            fee: 1,
            signature: vec![],
            timestamp: 0,
        };
        assert!(ledger.apply_transfer(&tx, "miner").is_err());
    }

    #[test]
    fn test_supply_cap_enforcement() {
        let mut ledger = Ledger::new("protocol".to_string());
        ledger
            .mint_tokens("alice", &TokenType::TribeChain, TRIBE_HARD_CAP)
            .unwrap();
        let result = ledger.mint_tokens("bob", &TokenType::TribeChain, 1);
        assert!(result.is_err(), "Should reject minting beyond hard cap");
    }

    #[test]
    fn test_coinbase_maturity() {
        let mut ledger = Ledger::new("protocol".to_string());
        assert!(!ledger.is_coinbase_mature("miner", 0, 50));
        assert!(ledger.is_coinbase_mature("miner", 0, 100));
        assert!(ledger.is_coinbase_mature("miner", 0, 200));
    }

    #[test]
    fn test_rollback_transfer() {
        let mut ledger = Ledger::new("protocol".to_string());
        ledger
            .mint_tokens("alice", &TokenType::TribeChain, 1000)
            .unwrap();
        let alice_before = ledger.balance_of("alice", &TokenType::TribeChain);
        let tx = TransferTransaction {
            tx_hash: [0u8; 32],
            nonce: 0,
            from: "alice".to_string(),
            to: "bob".to_string(),
            token: TokenType::TribeChain,
            amount: 200,
            fee: 1,
            signature: vec![],
            timestamp: 0,
        };
        ledger.apply_transfer(&tx, "miner").unwrap();
        ledger.rollback_transfer(&tx, "miner").unwrap();
        assert_eq!(
            ledger.balance_of("alice", &TokenType::TribeChain),
            alice_before
        );
        assert_eq!(ledger.balance_of("bob", &TokenType::TribeChain), 0);
        assert_eq!(ledger.balance_of("miner", &TokenType::TribeChain), 0);
    }

    #[test]
    fn test_current_nonce() {
        let mut ledger = Ledger::new("protocol".to_string());
        assert_eq!(ledger.current_nonce("alice"), 0);
        ledger
            .mint_tokens("alice", &TokenType::TribeChain, 1000)
            .unwrap();
        let tx = TransferTransaction {
            tx_hash: [0u8; 32],
            nonce: 0,
            from: "alice".to_string(),
            to: "bob".to_string(),
            token: TokenType::TribeChain,
            amount: 100,
            fee: 0,
            signature: vec![],
            timestamp: 0,
        };
        ledger.apply_transfer(&tx, "miner").unwrap();
        assert_eq!(ledger.current_nonce("alice"), 1);
    }

    #[test]
    fn test_total_supply_of() {
        let mut ledger = Ledger::new("protocol".to_string());
        assert_eq!(ledger.total_supply_of(&TokenType::TribeChain), 0);
        ledger
            .mint_tokens("alice", &TokenType::TribeChain, 500)
            .unwrap();
        assert_eq!(ledger.total_supply_of(&TokenType::TribeChain), 500);
    }

    #[test]
    fn test_apply_coinbase_supply_cap() {
        let mut ledger = Ledger::new("protocol".to_string());
        let cb = CoinbaseTransaction {
            tx_hash: [0u8; 32],
            height: 1,
            miner_address: "miner".to_string(),
            block_reward: TRIBE_HARD_CAP,
            proof_rewards: vec![],
            signature: vec![],
        };
        ledger.apply_coinbase(&cb).unwrap();
        let cb2 = CoinbaseTransaction {
            tx_hash: [1u8; 32],
            height: 2,
            miner_address: "miner".to_string(),
            block_reward: 1,
            proof_rewards: vec![],
            signature: vec![],
        };
        assert!(ledger.apply_coinbase(&cb2).is_err());
    }

    #[test]
    fn test_apply_rollback_block() {
        let mut ledger = Ledger::new("protocol".to_string());
        ledger
            .mint_tokens("alice", &TokenType::TribeChain, 1000)
            .unwrap();

        let tx1 = serde_json::to_value(TransferTransaction {
            tx_hash: [1u8; 32],
            nonce: 0,
            from: "alice".to_string(),
            to: "bob".to_string(),
            token: TokenType::TribeChain,
            amount: 100,
            fee: 5,
            signature: vec![],
            timestamp: 0,
        })
        .unwrap();

        let cb = serde_json::to_value(CoinbaseTransaction {
            tx_hash: [0u8; 32],
            height: 1,
            miner_address: "miner".to_string(),
            block_reward: 100,
            proof_rewards: vec![ProofRewardEntry {
                miner_pubkey: "prover1".to_string(),
                reward_amount: 10,
                proof_hash: "proof1".to_string(),
            }],
            signature: vec![],
        })
        .unwrap();

        let block = HexBlock {
            parent_hash: [0u8; 32],
            height: 1,
            tx_merkle_root: [0u8; 32],
            transactions: Some(vec![cb, tx1]),
            miner_address: Some("miner".to_string()),
            timestamp: 100,
            nonce: 0,
            coord: hexchain_p2p::lattice_geometry::HCPCoord { q: 0, r: 0, s: 0 },
            neighbor_hashes: [hexchain_p2p::types::NEIGHBOR_SLOT_EMPTY; 12],
            tensor: hexchain_p2p::types::TensorMeta {
                expected_capacity: 1000,
                actual_capacity: 1000,
                compression_num: 1,
                compression_den: 1,
            },
        };

        let receipts = ledger.apply_block(&block).unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(ledger.balance_of("alice", &TokenType::TribeChain), 895);
        assert_eq!(ledger.balance_of("bob", &TokenType::TribeChain), 100);
        assert!(ledger.balance_of("miner", &TokenType::TribeChain) >= 100);
        assert_eq!(ledger.block_height(), 1);

        ledger.rollback_block(&block).unwrap();
        assert_eq!(ledger.balance_of("alice", &TokenType::TribeChain), 1000);
        assert_eq!(ledger.balance_of("bob", &TokenType::TribeChain), 0);
    }
}
