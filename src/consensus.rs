use std::collections::HashMap;
use std::sync::Arc;

use pot_o_extensions::ExtensionRegistry;
use pot_o_mining::PotOConsensus;
use serde::Serialize;
use tokio::sync::RwLock;

use crate::config::ValidatorConfig;
use crate::device_registry::RegisteredDevice;

#[derive(Debug, Clone, Default, Serialize)]
pub struct ValidatorStats {
    pub total_challenges_issued: u64,
    pub total_proofs_received: u64,
    pub total_proofs_valid: u64,
    pub active_miners: u64,
    pub uptime_secs: u64,
    /// Paths validated in the current challenge round (reset on new challenge).
    pub paths_in_block: u64,
    /// Tensor computations completed in the current challenge round (reset on new challenge).
    pub calcs_in_block: u64,
}

pub struct AppState {
    pub config: ValidatorConfig,
    pub consensus: PotOConsensus,
    pub extensions: ExtensionRegistry,
    pub current_challenge: RwLock<Option<pot_o_mining::Challenge>>,
    pub stats: RwLock<ValidatorStats>,
    /// device_id (e.g. MAC) -> RegisteredDevice. Persisted so ESP mappings survive restarts.
    pub device_registry: RwLock<HashMap<String, RegisteredDevice>>,
    pub registry_path: String,
}

pub fn create_app_state(
    cfg: ValidatorConfig,
    consensus: PotOConsensus,
    extensions: ExtensionRegistry,
    registry_path: String,
    device_registry: HashMap<String, RegisteredDevice>,
) -> Arc<AppState> {
    Arc::new(AppState {
        config: cfg,
        consensus,
        extensions,
        current_challenge: RwLock::new(None),
        stats: RwLock::new(ValidatorStats::default()),
        device_registry: RwLock::new(device_registry),
        registry_path,
    })
}
