use pot_o_core::TribeResult;
use pot_o_mining::PotOProof;
use serde::{Deserialize, Serialize};

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
}

// ---------------------------------------------------------------------------
// NativeDevice (implemented now)
// ---------------------------------------------------------------------------

pub struct NativeDevice;

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
            uptime_secs: 0,
            last_heartbeat: chrono::Utc::now(),
        })
    }
}

// ---------------------------------------------------------------------------
// ESP32SDevice (stubbed)
// ---------------------------------------------------------------------------

pub struct ESP32SDevice;

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
        todo!("ESP32S heartbeat via HTTP/MQTT not yet implemented")
    }
}

// ---------------------------------------------------------------------------
// ESP8266Device (stubbed)
// ---------------------------------------------------------------------------

pub struct ESP8266Device;

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
        todo!("ESP8266 heartbeat via HTTP/MQTT not yet implemented")
    }
}

// ---------------------------------------------------------------------------
// WasmDevice (stubbed)
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
        todo!("WASM device heartbeat not yet implemented")
    }
}
