pub mod chain_bridge;
pub mod device_protocol;
pub mod peer_network;
pub mod pool_strategy;
pub mod security;

pub use chain_bridge::{ChainBridge, SolanaBridge};
pub use device_protocol::{
    DeviceProtocol, DeviceStatus, DeviceType, ESP32SDevice, ESP8266Device, NativeDevice,
    WasmDevice,
};
pub use peer_network::{LocalOnlyNetwork, PeerNetwork};
pub use pool_strategy::{PoolStrategy, SoloStrategy};
pub use security::{Ed25519Authority, ProofAuthority};

use serde::{Deserialize, Serialize};

/// Central registry that holds the active extension implementations.
/// Constructed once at startup from config/env, then passed by reference.
pub struct ExtensionRegistry {
    pub device: Box<dyn DeviceProtocol>,
    pub network: Box<dyn PeerNetwork>,
    pub pool: Box<dyn PoolStrategy>,
    pub chain: Box<dyn ChainBridge>,
    pub auth: Box<dyn ProofAuthority>,
}

impl ExtensionRegistry {
    /// Build the default registry for single-node local operation.
    pub fn local_defaults(solana_rpc_url: &str, program_id: &str) -> Self {
        Self {
            device: Box::new(NativeDevice::new()),
            network: Box::new(LocalOnlyNetwork::new()),
            pool: Box::new(SoloStrategy),
            chain: Box::new(SolanaBridge::new(
                solana_rpc_url.to_string(),
                program_id.to_string(),
            )),
            auth: Box::new(Ed25519Authority),
        }
    }
}
