pub mod chain_bridge;
pub mod device_protocol;
pub mod genesis;
pub mod gossip_client;
pub mod ledger;
pub mod marketplace;
pub mod mdns_discovery;
pub mod mempool;
pub mod messaging;
pub mod peer_network;
pub mod pool_strategy;
pub mod proof_trace;
pub mod rewards;
pub mod security;
pub mod state_root;
pub mod tx;

pub use chain_bridge::{ChainBridge, TribechainBridge};
pub use device_protocol::{
    DeviceProtocol, DeviceStatus, DeviceType, ESP32SDevice, ESP8266Device, NativeDevice, WasmDevice,
};
pub use gossip_client::GossipClient;
pub use ledger::{
    load_ledger, spawn_persist_ledger, Ledger, LedgerEntry, LedgerSnapshot, TxReceipt,
    DEFAULT_LEDGER_PATH,
};
pub use marketplace::{
    parse_market_asset, MarketAsset, Marketplace, Order, OrderBook, OrderSide, OrderStatus, Trade,
};
pub use mdns_discovery::{MdnsDiscovery, PeerDiscovery};
pub use mempool::Mempool;
pub use messaging::{Messaging, MinerMessage, ValidatorMessage};
pub use peer_network::{LocalOnlyNetwork, PeerNetwork, VpnMeshConfig, VpnMeshNetwork};
pub use pool_strategy::{
    MinerShare, PPLNSPool, PoolStrategy, PoolType, ProofRecord, ProportionalPool, SoloStrategy,
};
pub use rewards::calculate_mining_reward;
pub use security::{Ed25519Authority, ProofAuthority};

pub use crate::genesis::Genesis;
use hexchain_p2p::block_store::BlockStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ExtensionRegistry {
    pub device: Box<dyn DeviceProtocol>,
    pub network: Arc<dyn PeerNetwork>,
    pub pool: Box<dyn PoolStrategy>,
    pub chain: Box<dyn ChainBridge>,
    pub auth: Box<dyn ProofAuthority>,
    pub ledger: Arc<RwLock<Ledger>>,
    pub marketplace: Arc<RwLock<Marketplace>>,
    pub messaging: Arc<Messaging>,
    pub mempool: Option<Arc<Mempool>>,
    pub block_store: Option<Arc<BlockStore>>,
    pub genesis: Option<Genesis>,
    pub tribechain_enabled: bool,
}

impl ExtensionRegistry {
    pub fn local_defaults(protocol_fee_address: &str, marketplace_fee_bps: u64) -> Self {
        Self {
            device: Box::new(NativeDevice::new()),
            network: Arc::new(LocalOnlyNetwork::new()),
            pool: Box::new(SoloStrategy),
            chain: Box::new(TribechainBridge::new()),
            auth: Box::new(Ed25519Authority::new("")),
            ledger: Arc::new(RwLock::new(Ledger::new(protocol_fee_address.to_string()))),
            marketplace: Arc::new(RwLock::new(Marketplace::new(
                marketplace_fee_bps,
                protocol_fee_address.to_string(),
            ))),
            messaging: Messaging::new(),
            mempool: None,
            block_store: None,
            genesis: None,
            tribechain_enabled: false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_config(
        peer_network_mode: &str,
        pool_strategy: &str,
        device_protocol: &str,
        protocol_fee_address: &str,
        marketplace_fee_bps: u64,
        ledger: Option<Ledger>,
        bootstrap_urls: &[String],
        enable_mdns: bool,
        mdns_service_name: &str,
        peer_timeout_secs: u64,
        _challenge_relay_enabled: bool,
    ) -> Self {
        let network: Arc<dyn PeerNetwork> = match peer_network_mode {
            "vpn_mesh" => {
                let config = peer_network::VpnMeshConfig {
                    wireguard_interface: "wg0".into(),
                    peer_addresses: vec![],
                    mdns_enabled: enable_mdns,
                    gossip_port: 8765,
                };
                match VpnMeshNetwork::new(
                    uuid::Uuid::new_v4().to_string(),
                    config,
                    bootstrap_urls.to_vec(),
                    enable_mdns,
                    mdns_service_name,
                    peer_timeout_secs,
                ) {
                    Ok(network) => Arc::new(network),
                    Err(_) => Arc::new(LocalOnlyNetwork::new()),
                }
            }
            _ => Arc::new(LocalOnlyNetwork::new()),
        };

        let pool: Box<dyn PoolStrategy> = match pool_strategy {
            "proportional" => Box::new(ProportionalPool { min_stake: 1000 }),
            "pplns" => Box::new(PPLNSPool {
                window_size: 100,
                min_stake: 1000,
            }),
            _ => Box::new(SoloStrategy),
        };

        let device: Box<dyn DeviceProtocol> = match device_protocol {
            "esp32s" => Box::new(ESP32SDevice::new(uuid::Uuid::new_v4().to_string())),
            "esp8266" => Box::new(ESP8266Device::new(uuid::Uuid::new_v4().to_string())),
            "wasm" => Box::new(WasmDevice),
            _ => Box::new(NativeDevice::new()),
        };

        let ledger = ledger.unwrap_or_else(|| Ledger::new(protocol_fee_address.to_string()));

        Self {
            device,
            network,
            pool,
            chain: Box::new(TribechainBridge::new()),
            auth: Box::new(Ed25519Authority::new("")),
            ledger: Arc::new(RwLock::new(ledger)),
            marketplace: Arc::new(RwLock::new(Marketplace::new(
                marketplace_fee_bps,
                protocol_fee_address.to_string(),
            ))),
            messaging: Messaging::new(),
            mempool: None,
            block_store: None,
            genesis: None,
            tribechain_enabled: false,
        }
    }
}
