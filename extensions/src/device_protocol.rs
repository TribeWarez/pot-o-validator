use pot_o_core::TribeResult;
use pot_o_mining::PotOProof;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    Native,
    ESP32S,
    ESP8266,
    WASM,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub device_type: DeviceType,
    pub online: bool,
    pub uptime_secs: u64,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
}

/// How a mining device communicates with the validator.
pub trait DeviceProtocol: Send + Sync {
    fn device_type(&self) -> DeviceType;
    fn max_tensor_dims(&self) -> (usize, usize);
    fn max_working_memory(&self) -> usize;
    fn heartbeat(&self) -> TribeResult<DeviceStatus>;
    fn supported_operations(&self) -> Vec<&'static str>;
}

// ---------------------------------------------------------------------------
// NativeDevice
// ---------------------------------------------------------------------------

pub struct NativeDevice {
    started_at: Instant,
}

impl NativeDevice {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
        }
    }
}

impl Default for NativeDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceProtocol for NativeDevice {
    fn device_type(&self) -> DeviceType {
        DeviceType::Native
    }
    fn max_tensor_dims(&self) -> (usize, usize) {
        (1024, 1024)
    }
    fn max_working_memory(&self) -> usize {
        1024 * 1024 * 512 // 512 MB
    }
    fn heartbeat(&self) -> TribeResult<DeviceStatus> {
        Ok(DeviceStatus {
            device_type: DeviceType::Native,
            online: true,
            uptime_secs: self.started_at.elapsed().as_secs(),
            last_heartbeat: chrono::Utc::now(),
        })
    }
    fn supported_operations(&self) -> Vec<&'static str> {
        vec![
            "matrix_multiply",
            "convolution",
            "relu",
            "sigmoid",
            "tanh",
            "dot_product",
            "normalize",
        ]
    }
}

// ---------------------------------------------------------------------------
// ESP32SDevice
// ---------------------------------------------------------------------------

/// ESP32-S device protocol handler.
/// Tracks registered devices by ID and provides heartbeat status.
/// The actual mining runs on the ESP firmware; this represents
/// the validator-side view of a connected ESP32-S.
pub struct ESP32SDevice {
    pub device_id: String,
    started_at: Instant,
    last_seen: AtomicU64,
}

impl ESP32SDevice {
    pub fn new(device_id: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            device_id,
            started_at: Instant::now(),
            last_seen: AtomicU64::new(now),
        }
    }

    pub fn record_heartbeat(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_seen.store(now, Ordering::Relaxed);
    }

    pub fn is_stale(&self, timeout_secs: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last = self.last_seen.load(Ordering::Relaxed);
        now.saturating_sub(last) > timeout_secs
    }
}

impl DeviceProtocol for ESP32SDevice {
    fn device_type(&self) -> DeviceType {
        DeviceType::ESP32S
    }
    fn max_tensor_dims(&self) -> (usize, usize) {
        (64, 64)
    }
    fn max_working_memory(&self) -> usize {
        320 * 1024 // 320 KB
    }
    fn heartbeat(&self) -> TribeResult<DeviceStatus> {
        self.record_heartbeat();
        Ok(DeviceStatus {
            device_type: DeviceType::ESP32S,
            online: !self.is_stale(90),
            uptime_secs: self.started_at.elapsed().as_secs(),
            last_heartbeat: chrono::Utc::now(),
        })
    }
    fn supported_operations(&self) -> Vec<&'static str> {
        vec![
            "matrix_multiply",
            "convolution",
            "relu",
            "sigmoid",
            "dot_product",
            "normalize",
        ]
    }
}

// ---------------------------------------------------------------------------
// ESP8266Device
// ---------------------------------------------------------------------------

/// ESP8266 device protocol handler.
/// Reduced tensor dimensions (32x32) and limited operation set.
pub struct ESP8266Device {
    pub device_id: String,
    started_at: Instant,
    last_seen: AtomicU64,
}

impl ESP8266Device {
    pub fn new(device_id: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            device_id,
            started_at: Instant::now(),
            last_seen: AtomicU64::new(now),
        }
    }

    pub fn record_heartbeat(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_seen.store(now, Ordering::Relaxed);
    }

    pub fn is_stale(&self, timeout_secs: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last = self.last_seen.load(Ordering::Relaxed);
        now.saturating_sub(last) > timeout_secs
    }
}

impl DeviceProtocol for ESP8266Device {
    fn device_type(&self) -> DeviceType {
        DeviceType::ESP8266
    }
    fn max_tensor_dims(&self) -> (usize, usize) {
        (32, 32)
    }
    fn max_working_memory(&self) -> usize {
        80 * 1024 // 80 KB
    }
    fn heartbeat(&self) -> TribeResult<DeviceStatus> {
        self.record_heartbeat();
        Ok(DeviceStatus {
            device_type: DeviceType::ESP8266,
            online: !self.is_stale(90),
            uptime_secs: self.started_at.elapsed().as_secs(),
            last_heartbeat: chrono::Utc::now(),
        })
    }
    fn supported_operations(&self) -> Vec<&'static str> {
        vec!["relu", "sigmoid", "dot_product", "normalize"]
    }
}

// ---------------------------------------------------------------------------
// WasmDevice (stubbed – pending wasm-bindgen integration)
// ---------------------------------------------------------------------------

pub struct WasmDevice;

impl DeviceProtocol for WasmDevice {
    fn device_type(&self) -> DeviceType {
        DeviceType::WASM
    }
    fn max_tensor_dims(&self) -> (usize, usize) {
        (256, 256)
    }
    fn max_working_memory(&self) -> usize {
        64 * 1024 * 1024 // 64 MB WASM linear memory
    }
    fn heartbeat(&self) -> TribeResult<DeviceStatus> {
        Ok(DeviceStatus {
            device_type: DeviceType::WASM,
            online: false,
            uptime_secs: 0,
            last_heartbeat: chrono::Utc::now(),
        })
    }
    fn supported_operations(&self) -> Vec<&'static str> {
        vec![
            "matrix_multiply",
            "convolution",
            "relu",
            "sigmoid",
            "tanh",
            "dot_product",
            "normalize",
        ]
    }
}
