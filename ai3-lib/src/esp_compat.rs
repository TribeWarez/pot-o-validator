use crate::tensor::{Tensor, TensorData, TensorShape};
use pot_o_core::TribeResult;
use serde::{Deserialize, Serialize};

/// ESP device types supported by the mining network (from .AI3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ESPDeviceType {
    ESP32,
    ESP32S,
    ESP8266,
}

impl ESPDeviceType {
    pub fn max_tensor_dims(&self) -> (usize, usize) {
        match self {
            Self::ESP32 | Self::ESP32S => (64, 64),
            Self::ESP8266 => (32, 32),
        }
    }

    pub fn max_working_memory(&self) -> usize {
        match self {
            Self::ESP32 | Self::ESP32S => 320 * 1024, // 320 KB
            Self::ESP8266 => 80 * 1024,               // 80 KB
        }
    }

    pub fn supported_operations(&self) -> Vec<&'static str> {
        match self {
            Self::ESP32 | Self::ESP32S => vec![
                "matrix_multiply",
                "convolution",
                "relu",
                "sigmoid",
                "dot_product",
                "normalize",
            ],
            Self::ESP8266 => vec!["relu", "sigmoid", "dot_product", "normalize"],
        }
    }
}

impl std::str::FromStr for ESPDeviceType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "esp32" => Ok(Self::ESP32),
            "esp32s" => Ok(Self::ESP32S),
            "esp8266" => Ok(Self::ESP8266),
            _ => Err(format!("Unknown ESP device type: {s}")),
        }
    }
}

/// Mining configuration for ESP devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ESPMiningConfig {
    pub device_type: ESPDeviceType,
    pub wifi_ssid: String,
    pub rpc_host: String,
    pub rpc_port: u16,
    pub max_tensor_dim: usize,
    pub heartbeat_interval_ms: u64,
}

impl ESPMiningConfig {
    pub fn for_device(device_type: ESPDeviceType) -> Self {
        let (max_dim, _) = device_type.max_tensor_dims();
        Self {
            device_type,
            wifi_ssid: String::new(),
            rpc_host: "pot.rpc.gateway.tribewarez.com".into(),
            rpc_port: 8900,
            max_tensor_dim: max_dim,
            heartbeat_interval_ms: 30_000,
        }
    }
}

/// Utility for checking and optimizing tensors for ESP device constraints
pub struct ESPCompatibility;

impl ESPCompatibility {
    pub fn get_recommended_config(device_type: ESPDeviceType) -> ESPMiningConfig {
        ESPMiningConfig::for_device(device_type)
    }

    /// Check whether a tensor fits within a device's memory constraints
    pub fn fits_device(tensor: &Tensor, device_type: ESPDeviceType) -> bool {
        let max_mem = device_type.max_working_memory();
        let (max_r, max_c) = device_type.max_tensor_dims();
        let within_mem = tensor.byte_size() <= max_mem;
        let within_dims = tensor.shape.dims.iter().all(|&d| d <= max_r.max(max_c));
        within_mem && within_dims
    }

    /// Clamp a tensor to fit within ESP device limits
    pub fn optimize_for_esp(
        tensor: &Tensor,
        device_type: &ESPDeviceType,
    ) -> TribeResult<Tensor> {
        let (max_r, _max_c) = device_type.max_tensor_dims();
        Ok(tensor.clamp_dimensions(max_r))
    }

    /// Return the most restrictive tensor dimension across a set of device types
    pub fn most_restrictive_dim(devices: &[ESPDeviceType]) -> usize {
        devices
            .iter()
            .map(|d| {
                let (r, c) = d.max_tensor_dims();
                r.min(c)
            })
            .min()
            .unwrap_or(pot_o_core::ESP_MAX_TENSOR_DIM)
    }
}
