use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genesis {
    pub chain_id: String,
}

impl Genesis {
    pub fn load(path: &str) -> Self {
        let data = std::fs::read_to_string(path).expect("Failed to read genesis file");
        serde_json::from_str(&data).expect("Failed to parse genesis JSON")
    }
}
