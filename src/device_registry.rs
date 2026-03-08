//! Device registry: persistence and in-memory state for miners (ESP, CPU, GPU) and real-time progress.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::ValidatorConfig;

/// Current running calculation reported by a device/thread in real time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentCalculation {
    pub challenge_id: String,
    pub hash: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Per-device record for the registry (device_id -> device state).
/// When device_id is set on submit, one registry entry per device; optional miner_pubkeys
/// tracks all miner pubkeys that have submitted from this device for analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredDevice {
    pub device_type: String,
    pub node_id: String,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub proofs_valid: u64,
    pub tasks_processed: u64,
    /// Miner pubkeys that have submitted from this device (when keyed by device_id). Capped for storage.
    #[serde(default)]
    pub miner_pubkeys: Vec<String>,
    /// Current running calculation with hash, sent in real time by device/thread.
    #[serde(default)]
    pub current_calculation: Option<CurrentCalculation>,
}

/// In-memory map: registry key (device_id or miner_pubkey:device_type) -> device record.
pub type DeviceRegistry = HashMap<String, RegisteredDevice>;

/// Logical interface for device registry persistence and queries.
#[allow(dead_code)]
pub trait DeviceStore: Send + Sync {
    fn load(&self) -> DeviceRegistry;
    fn persist(&self, registry: &DeviceRegistry);
}

/// Canonical device type labels used in the registry and API.
pub const DEVICE_TYPE_KEYS: &[&str] = &["esp32", "esp8266", "gpu", "cpu", "native", "wasm"];

/// Default path for the JSON device registry file.
pub const DEFAULT_REGISTRY_PATH: &str = "device_registry.json";

/// Normalizes a device type string to one of [`DEVICE_TYPE_KEYS`] (e.g. esp32s -> esp32).
pub fn normalize_device_type(s: &str) -> String {
    let lower = s.to_lowercase();
    if lower == "esp32" || lower == "esp32s" {
        return "esp32".to_string();
    }
    if lower == "esp8266" {
        return "esp8266".to_string();
    }
    if lower == "gpu" || lower == "cuda" || lower == "opencl" {
        return "gpu".to_string();
    }
    if lower == "cpu" {
        return "cpu".to_string();
    }
    if lower == "native" {
        return "native".to_string();
    }
    if lower == "wasm" {
        return "wasm".to_string();
    }
    "native".to_string()
}

/// Loads the device registry from a JSON file; returns an empty map if the file is missing or invalid.
pub fn load_registry(path: &str) -> DeviceRegistry {
    match std::fs::read_to_string(path) {
        Ok(data) => {
            let reg: DeviceRegistry = serde_json::from_str(&data).unwrap_or_default();
            tracing::debug!(path, count = reg.len(), "Loaded device registry from disk");
            reg
        }
        Err(_) => {
            tracing::debug!(path, "No existing device registry file; starting empty");
            HashMap::new()
        }
    }
}

/// Spawns a background task to write the registry to disk as pretty JSON.
pub fn spawn_persist_registry(reg: DeviceRegistry, path: String) {
    let count = reg.len();
    tokio::spawn(async move {
        if let Ok(json) = serde_json::to_string_pretty(&reg) {
            if let Err(e) = tokio::fs::write(&path, json).await {
                tracing::warn!(path, error = %e, "Failed to persist device registry");
            } else {
                tracing::debug!(path, count, "Persisted device registry");
            }
        }
    });
}

/// Simple JSON-on-disk implementation of `DeviceStore` backed by a single file.
#[allow(dead_code)]
pub struct JsonFileDeviceStore {
    path: String,
    node_id: String,
}

impl JsonFileDeviceStore {
    /// Creates a store that uses the given path and node_id from config.
    #[allow(dead_code)]
    pub fn new_from_config(cfg: &ValidatorConfig, path: String) -> Self {
        Self {
            path,
            node_id: cfg.node_id.clone(),
        }
    }
}

impl DeviceStore for JsonFileDeviceStore {
    fn load(&self) -> DeviceRegistry {
        load_registry(&self.path)
    }

    fn persist(&self, registry: &DeviceRegistry) {
        let cloned = registry.clone();
        spawn_persist_registry(cloned, self.path.clone());
    }
}
