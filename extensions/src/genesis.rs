use pot_o_core::TokenType;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisEntry {
    pub address: String,
    pub token: TokenType,
    pub balance: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genesis {
    pub entries: Vec<GenesisEntry>,
    pub chain_id: String,
    pub created_at: u64,
    pub tribechain_genesis_version: u64,
}

impl Genesis {
    pub fn load(path: &str) -> Result<Self, String> {
        let data = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read genesis file '{}': {}", path, e))?;
        serde_json::from_str(&data).map_err(|e| format!("Failed to parse genesis JSON: {}", e))
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut seen = HashSet::new();
        for entry in &self.entries {
            if entry.balance == 0 {
                return Err(format!("Zero balance for address {}", entry.address));
            }
            let key = (entry.address.clone(), entry.token.clone());
            if !seen.insert(key) {
                return Err(format!(
                    "Duplicate entry: {} / {:?}",
                    entry.address, entry.token
                ));
            }
        }
        Ok(())
    }

    pub fn apply_to_ledger(&self, ledger: &mut super::ledger::Ledger) -> Result<(), String> {
        for entry in &self.entries {
            ledger.issue(&entry.address, &entry.token, entry.balance);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_validate_no_duplicates() {
        let genesis = Genesis {
            entries: vec![
                GenesisEntry {
                    address: "alice".to_string(),
                    token: TokenType::TribeChain,
                    balance: 1000,
                },
                GenesisEntry {
                    address: "alice".to_string(),
                    token: TokenType::TribeChain,
                    balance: 500,
                },
            ],
            chain_id: "test".to_string(),
            created_at: 0,
            tribechain_genesis_version: 1,
        };
        assert!(genesis.validate().is_err());
    }

    #[test]
    fn test_genesis_validate_no_zero_balance() {
        let genesis = Genesis {
            entries: vec![GenesisEntry {
                address: "alice".to_string(),
                token: TokenType::TribeChain,
                balance: 0,
            }],
            chain_id: "test".to_string(),
            created_at: 0,
            tribechain_genesis_version: 1,
        };
        assert!(genesis.validate().is_err());
    }

    #[test]
    fn test_genesis_validate_ok() {
        let genesis = Genesis {
            entries: vec![
                GenesisEntry {
                    address: "alice".to_string(),
                    token: TokenType::TribeChain,
                    balance: 1000,
                },
                GenesisEntry {
                    address: "bob".to_string(),
                    token: TokenType::TribeChain,
                    balance: 500,
                },
            ],
            chain_id: "test".to_string(),
            created_at: 0,
            tribechain_genesis_version: 1,
        };
        assert!(genesis.validate().is_ok());
    }

    #[test]
    fn test_genesis_apply_to_ledger() {
        let mut ledger = super::super::ledger::Ledger::new("fee".to_string());
        let genesis = Genesis {
            entries: vec![
                GenesisEntry {
                    address: "alice".to_string(),
                    token: TokenType::TribeChain,
                    balance: 1000,
                },
                GenesisEntry {
                    address: "bob".to_string(),
                    token: TokenType::TribeChain,
                    balance: 500,
                },
            ],
            chain_id: "test".to_string(),
            created_at: 0,
            tribechain_genesis_version: 1,
        };
        genesis.apply_to_ledger(&mut ledger).unwrap();
        assert_eq!(ledger.balance_of("alice", &TokenType::TribeChain), 1000);
        assert_eq!(ledger.balance_of("bob", &TokenType::TribeChain), 500);
    }
}
