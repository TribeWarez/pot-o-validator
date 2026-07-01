use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genesis {
    pub chain_id: String,
}

impl Genesis {
    pub fn load(path: &str) -> Result<Self, String> {
        let data = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read genesis file '{}': {}", path, e))?;
        serde_json::from_str(&data).map_err(|e| format!("Failed to parse genesis JSON: {}", e))
    }
}
