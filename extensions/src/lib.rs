//! PoT-O extensions: chain bridge, DeFi client, device protocol, peer network, pool strategy,
//! proof authority, and token ledger.

pub mod chain_bridge;
pub mod defi;
pub mod device_protocol;
pub mod gossip_client;
pub mod ledger;
pub mod marketplace;
pub mod mdns_discovery;
pub mod peer_network;
pub mod pool_strategy;
pub mod security;

pub use chain_bridge::{ChainBridge, SolanaBridge};
pub use defi::{
    DefiClient, EscrowInfo, LiquidityPoolInfo, StakeAccountInfo, StakingPoolInfo, SwapQuoteInfo,
    TreasuryInfo, UserVaultInfo,
};
pub use device_protocol::{
    DeviceProtocol, DeviceStatus, DeviceType, ESP32SDevice, ESP8266Device, NativeDevice, WasmDevice,
};
pub use gossip_client::GossipClient;
pub use ledger::{
    load_ledger, spawn_persist_ledger, Ledger, LedgerEntry, TxReceipt, DEFAULT_LEDGER_PATH,
};
pub use marketplace::{
    parse_market_asset, MarketAsset, Marketplace, Order, OrderBook, OrderSide, OrderStatus, Trade,
};
pub use mdns_discovery::{MdnsDiscovery, PeerDiscovery};
pub use peer_network::{LocalOnlyNetwork, PeerNetwork, VpnMeshConfig, VpnMeshNetwork};
pub use pool_strategy::{PPLNSPool, PoolStrategy, PoolType, ProportionalPool, SoloStrategy};
pub use security::{Ed25519Authority, ProofAuthority};

use std::sync::Arc;
use tokio::sync::RwLock;

/// Central registry that holds the active extension implementations.
/// Constructed once at startup from config/env, then passed by reference.
pub struct ExtensionRegistry {
    pub device: Box<dyn DeviceProtocol>,
    pub network: Box<dyn PeerNetwork>,
    pub pool: Box<dyn PoolStrategy>,
    pub chain: Box<dyn ChainBridge>,
    pub auth: Box<dyn ProofAuthority>,
    pub ledger: Arc<RwLock<Ledger>>,
    pub marketplace: Arc<RwLock<Marketplace>>,
}

impl ExtensionRegistry {
    /// Build the default registry for single-node local operation.
    pub fn local_defaults(
        solana_rpc_url: &str,
        program_id: &str,
        relayer_keypair_path: &str,
        auto_register_miners: bool,
    ) -> Self {
        Self {
            device: Box::new(NativeDevice::new()),
            network: Box::new(LocalOnlyNetwork::new()),
            pool: Box::new(SoloStrategy),
            chain: Box::new(SolanaBridge::new(
                solana_rpc_url.to_string(),
                program_id.to_string(),
                relayer_keypair_path.to_string(),
                auto_register_miners,
            )),
            auth: Box::new(Ed25519Authority),
            ledger: Arc::new(RwLock::new(Ledger::new(String::new()))),
            marketplace: Arc::new(RwLock::new(Marketplace::new(25, String::new()))),
        }
    }

    /// Build the registry from config strings specifying peer network mode and pool strategy.
    ///
    /// # Arguments
    /// * `solana_rpc_url` - Solana RPC endpoint URL
    /// * `program_id` - PoT-O program ID
    /// * `relayer_keypair_path` - Path to relayer keypair
    /// * `auto_register_miners` - Whether to auto-register miners
    /// * `peer_network_mode` - Network mode: "local_only" or "vpn_mesh" (defaults to "local_only" if unknown)
    /// * `pool_strategy` - Pool strategy: "solo", "proportional", or "pplns" (defaults to "solo" if unknown)
    #[allow(clippy::too_many_arguments)]
    pub fn from_config(
        solana_rpc_url: &str,
        program_id: &str,
        relayer_keypair_path: &str,
        auto_register_miners: bool,
        peer_network_mode: &str,
        pool_strategy: &str,
        protocol_fee_address: &str,
        marketplace_fee_bps: u64,
        ledger: Option<Ledger>,
    ) -> Self {
        // Parse network mode
        let network: Box<dyn PeerNetwork> = match peer_network_mode {
            "vpn_mesh" => {
                let config = peer_network::VpnMeshConfig {
                    wireguard_interface: "wg0".into(),
                    peer_addresses: vec![],
                    mdns_enabled: true,
                    gossip_port: 8765,
                };
                match VpnMeshNetwork::new(
                    uuid::Uuid::new_v4().to_string(),
                    config,
                    vec![],
                    true,
                    "pot-o-validator",
                    30,
                ) {
                    Ok(network) => Box::new(network),
                    Err(_) => Box::new(LocalOnlyNetwork::new()), // Fallback on error
                }
            }
            _ => Box::new(LocalOnlyNetwork::new()), // Default: local_only
        };

        // Parse pool strategy
        let pool: Box<dyn PoolStrategy> = match pool_strategy {
            "proportional" => Box::new(ProportionalPool { min_stake: 1000 }),
            "pplns" => Box::new(PPLNSPool {
                window_size: 100,
                min_stake: 1000,
            }),
            _ => Box::new(SoloStrategy), // Default: solo
        };

        let ledger = ledger.unwrap_or_else(|| Ledger::new(protocol_fee_address.to_string()));

        Self {
            device: Box::new(NativeDevice::new()),
            network,
            pool,
            chain: Box::new(SolanaBridge::new(
                solana_rpc_url.to_string(),
                program_id.to_string(),
                relayer_keypair_path.to_string(),
                auto_register_miners,
            )),
            auth: Box::new(Ed25519Authority),
            ledger: Arc::new(RwLock::new(ledger)),
            marketplace: Arc::new(RwLock::new(Marketplace::new(
                marketplace_fee_bps,
                protocol_fee_address.to_string(),
            ))),
        }
    }
}
