//! Application state and validator stats shared across HTTP handlers.

use std::collections::HashMap;
use std::sync::Arc;

use pot_o_extensions::ExtensionRegistry;
use pot_o_mining::PotOConsensus;
use serde::Serialize;
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
}

/// Builds the shared application state used by the Axum router.
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
