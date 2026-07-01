//! Validator configuration: TOML file and environment overrides.

use serde::Deserialize;

/// Runtime configuration for the PoT-O validator (HTTP server, consensus, extensions).
#[derive(Debug, Clone, Deserialize)]
pub struct ValidatorConfig {
    /// Unique node identifier (default: random UUID).
    #[serde(default = "default_node_id")]
    pub node_id: String,
    /// Bind address for the HTTP server (default: 0.0.0.0).
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,
    /// HTTP port (default: 8900).
    #[serde(default = "default_port")]
    pub port: u16,
    /// Solana RPC URL for chain and program calls.
    #[serde(default = "default_solana_rpc_url")]
    pub solana_rpc_url: String,
    /// PoT-O program ID (optional; empty uses default).
    #[serde(default)]
    pub pot_program_id: String,
    /// Mining difficulty (target path count).
    #[serde(default = "default_difficulty")]
    pub difficulty: u64,
    /// Maximum tensor dimension for challenges.
    #[serde(default = "default_max_tensor_dim")]
    pub max_tensor_dim: usize,
    /// Max iterations per mining attempt.
    #[serde(default = "default_max_mine_iterations")]
    #[allow(dead_code)]
    pub max_mine_iterations: u64,
    /// Peer network mode (e.g. local_only).
    #[serde(default = "default_peer_network_mode")]
    pub peer_network_mode: String,
    /// Pool strategy (e.g. solo).
    #[serde(default = "default_pool_strategy")]
    pub pool_strategy: String,
    /// Chain bridge identifier (e.g. solana).
    #[serde(default = "default_chain_bridge")]
    pub chain_bridge: String,
    /// Device protocol (e.g. native).
    #[serde(default = "default_device_protocol")]
    pub device_protocol: String,
    /// Path to relayer keypair for on-chain miner registration.
    #[serde(default = "default_relayer_keypair_path")]
    pub relayer_keypair_path: String,
    /// Whether to auto-register miners on device registration when not yet on-chain.
    #[serde(default = "default_auto_register_miners")]
    pub auto_register_miners: bool,
    /// Bootstrap URLs for P2P discovery (default: empty list).
    #[serde(default = "default_bootstrap_urls")]
    pub bootstrap_urls: Vec<String>,
    /// Enable mDNS discovery (default: false).
    #[serde(default = "default_enable_mdns")]
    pub enable_mdns: bool,
    /// Service name for mDNS registration (default: "pot-o-validator").
    #[serde(default = "default_mdns_service_name")]
    pub mdns_service_name: String,
    /// Port for internal peer-to-peer API (default: 8900).
    #[serde(default = "default_internal_api_port")]
    pub internal_api_port: u16,
    /// Timeout for peer communication in seconds (default: 30).
    #[serde(default = "default_peer_timeout_secs")]
    pub peer_timeout_secs: u64,
    /// Enable push/pull challenge gossip (default: true).
    #[serde(default = "default_challenge_relay_enabled")]
    pub challenge_relay_enabled: bool,
    /// URL of primary validator for on-chain submission (default: "http://localhost:8899").
    #[serde(default = "default_primary_validator_url")]
    pub primary_validator_url: String,
    /// Maturity depth for hexchain consensus (default: 10).
    #[serde(default = "default_maturity_depth")]
    pub maturity_depth: u64,
    /// Symmetry numerator for hexchain consensus (default: 1).
    #[serde(default = "default_symmetry_num")]
    pub symmetry_num: u64,
    /// Symmetry denominator for hexchain consensus (default: 1).
    #[serde(default = "default_symmetry_den")]
    pub symmetry_den: u64,
    /// Base target for hexchain consensus (hex-encoded 32 bytes, default: all 0xFF).
    #[serde(default = "default_base_target")]
    pub base_target: String,
    /// Address that collects protocol fees from token transfers (default: empty = no fee).
    #[serde(default)]
    pub protocol_fee_address: String,
    /// Marketplace fee in basis points (e.g. 25 = 0.25%).
    #[serde(default = "default_marketplace_fee_bps")]
    pub marketplace_fee_bps: u64,
    /// Path to the TRIBE mint Ed25519 keypair JSON file (auto-generated if missing).
    #[serde(default = "default_tribe_mint_keypair_path")]
    pub tribe_mint_keypair_path: String,
    /// Enable TribeChain sidechain (mempool, block store, genesis).
    #[serde(default = "default_tribechain_enabled")]
    pub tribechain_enabled: bool,
    /// Minimum fee for TribeChain transactions.
    #[serde(default = "default_tribechain_min_fee")]
    pub tribechain_min_fee: u64,
    /// Maximum number of pending transactions in the mempool.
    #[serde(default = "default_tribechain_max_pool_size")]
    pub tribechain_max_pool_size: usize,
    /// Maximum transactions per block.
    #[serde(default = "default_tribechain_max_txs_per_block")]
    pub tribechain_max_txs_per_block: usize,
    /// Path to genesis JSON file.
    #[serde(default = "default_tribechain_genesis_path")]
    pub tribechain_genesis_path: String,
    /// Miner address for TribeChain block rewards.
    #[serde(default = "default_tribechain_miner_address")]
    pub tribechain_miner_address: String,
    /// Path to the block store persistence file.
    #[serde(default = "default_tribechain_blockstore_path")]
    pub tribechain_blockstore_path: String,
}

fn default_node_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn default_listen_addr() -> String {
    "0.0.0.0".into()
}
fn default_port() -> u16 {
    8900
}
fn default_solana_rpc_url() -> String {
    "http://testnet-solana-rpc-gateway:8899".into()
}
fn default_difficulty() -> u64 {
    2
}
fn default_max_tensor_dim() -> usize {
    64
}
fn default_max_mine_iterations() -> u64 {
    10_000
}
fn default_peer_network_mode() -> String {
    "local_only".into()
}
fn default_pool_strategy() -> String {
    "solo".into()
}
fn default_chain_bridge() -> String {
    "solana".into()
}
fn default_device_protocol() -> String {
    "native".into()
}
fn default_relayer_keypair_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    format!("{}/.config/solana/id.json", home)
}
fn default_auto_register_miners() -> bool {
    true
}
fn default_bootstrap_urls() -> Vec<String> {
    Vec::new()
}
fn default_enable_mdns() -> bool {
    false
}
fn default_mdns_service_name() -> String {
    "pot-o-validator".into()
}
fn default_internal_api_port() -> u16 {
    8900
}
fn default_peer_timeout_secs() -> u64 {
    30
}
fn default_challenge_relay_enabled() -> bool {
    true
}
fn default_primary_validator_url() -> String {
    "http://localhost:8899".into()
}
fn default_maturity_depth() -> u64 {
    10
}
fn default_symmetry_num() -> u64 {
    1
}
fn default_symmetry_den() -> u64 {
    1
}
fn default_base_target() -> String {
    "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".into()
}
fn default_protocol_fee_address() -> String {
    String::new()
}
fn default_marketplace_fee_bps() -> u64 {
    25
}
fn default_tribe_mint_keypair_path() -> String {
    "tribe_mint_keypair.json".into()
}
fn default_tribechain_enabled() -> bool {
    false
}
fn default_tribechain_min_fee() -> u64 {
    0
}
fn default_tribechain_max_pool_size() -> usize {
    10_000
}
fn default_tribechain_max_txs_per_block() -> usize {
    1000
}
fn default_tribechain_genesis_path() -> String {
    String::new()
}
fn default_tribechain_miner_address() -> String {
    String::new()
}
fn default_tribechain_blockstore_path() -> String {
    "blockstore.json".to_string()
}

impl ValidatorConfig {
    /// Load config from TOML file, then override with env vars.
    pub fn load() -> Self {
        let mut cfg: Self = std::fs::read_to_string("/config/default.toml")
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .or_else(|| {
                std::fs::read_to_string("config/default.toml")
                    .ok()
                    .and_then(|s| toml::from_str(&s).ok())
            })
            .unwrap_or_else(Self::defaults);

        // Env overrides
        if let Ok(v) = std::env::var("SOLANA_RPC_URL") {
            cfg.solana_rpc_url = v;
        }
        if let Ok(v) = std::env::var("POT_PROGRAM_ID") {
            cfg.pot_program_id = v;
        }
        if let Ok(v) = std::env::var("POT_O_DIFFICULTY") {
            if let Ok(d) = v.parse() {
                cfg.difficulty = d;
            }
        }
        if let Ok(v) = std::env::var("PORT") {
            if let Ok(p) = v.parse() {
                cfg.port = p;
            }
        }
        if let Ok(v) = std::env::var("PEER_NETWORK_MODE") {
            cfg.peer_network_mode = v;
        }
        if let Ok(v) = std::env::var("POOL_STRATEGY") {
            cfg.pool_strategy = v;
        }
        if let Ok(v) = std::env::var("CHAIN_BRIDGE") {
            cfg.chain_bridge = v;
        }
        if let Ok(v) = std::env::var("DEVICE_PROTOCOL") {
            cfg.device_protocol = v;
        }
        if let Ok(v) = std::env::var("RELAYER_KEYPAIR_PATH") {
            cfg.relayer_keypair_path = v;
        }
        if let Ok(v) = std::env::var("AUTO_REGISTER_MINERS") {
            cfg.auto_register_miners = v != "0" && v.to_lowercase() != "false";
        }
        if let Ok(v) = std::env::var("BOOTSTRAP_URLS") {
            cfg.bootstrap_urls = v.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Ok(v) = std::env::var("ENABLE_MDNS") {
            cfg.enable_mdns = v != "0" && v.to_lowercase() != "false";
        }
        if let Ok(v) = std::env::var("MDNS_SERVICE_NAME") {
            cfg.mdns_service_name = v;
        }
        if let Ok(v) = std::env::var("INTERNAL_API_PORT") {
            if let Ok(p) = v.parse() {
                cfg.internal_api_port = p;
            }
        }
        if let Ok(v) = std::env::var("PEER_TIMEOUT_SECS") {
            if let Ok(t) = v.parse() {
                cfg.peer_timeout_secs = t;
            }
        }
        if let Ok(v) = std::env::var("CHALLENGE_RELAY_ENABLED") {
            cfg.challenge_relay_enabled = v != "0" && v.to_lowercase() != "false";
        }
        if let Ok(v) = std::env::var("PRIMARY_VALIDATOR_URL") {
            cfg.primary_validator_url = v;
        }
        if let Ok(v) = std::env::var("MATURITY_DEPTH") {
            if let Ok(d) = v.parse() {
                cfg.maturity_depth = d;
            }
        }
        if let Ok(v) = std::env::var("SYMMETRY_NUM") {
            if let Ok(n) = v.parse() {
                cfg.symmetry_num = n;
            }
        }
        if let Ok(v) = std::env::var("SYMMETRY_DEN") {
            if let Ok(d) = v.parse() {
                cfg.symmetry_den = d;
            }
        }
        if let Ok(v) = std::env::var("BASE_TARGET") {
            cfg.base_target = v;
        }
        if let Ok(v) = std::env::var("PROTOCOL_FEE_ADDRESS") {
            cfg.protocol_fee_address = v;
        }
        if let Ok(v) = std::env::var("MARKETPLACE_FEE_BPS") {
            if let Ok(bps) = v.parse() {
                cfg.marketplace_fee_bps = bps;
            }
        }
        if let Ok(v) = std::env::var("TRIBE_MINT_KEYPAIR_PATH") {
            cfg.tribe_mint_keypair_path = v;
        }
        if let Ok(v) = std::env::var("TRIBECHAIN_ENABLED") {
            cfg.tribechain_enabled = v != "0" && v.to_lowercase() != "false";
        }
        if let Ok(v) = std::env::var("TRIBECHAIN_MIN_FEE") {
            if let Ok(f) = v.parse() {
                cfg.tribechain_min_fee = f;
            }
        }
        if let Ok(v) = std::env::var("TRIBECHAIN_MAX_POOL_SIZE") {
            if let Ok(s) = v.parse() {
                cfg.tribechain_max_pool_size = s;
            }
        }
        if let Ok(v) = std::env::var("TRIBECHAIN_MAX_TXS_PER_BLOCK") {
            if let Ok(n) = v.parse() {
                cfg.tribechain_max_txs_per_block = n;
            }
        }
        if let Ok(v) = std::env::var("TRIBECHAIN_GENESIS_PATH") {
            cfg.tribechain_genesis_path = v;
        }
        if let Ok(v) = std::env::var("TRIBECHAIN_MINER_ADDRESS") {
            cfg.tribechain_miner_address = v;
        }
        if let Ok(v) = std::env::var("TRIBECHAIN_BLOCKSTORE_PATH") {
            cfg.tribechain_blockstore_path = v;
        }

        cfg
    }

    fn defaults() -> Self {
        Self {
            node_id: default_node_id(),
            listen_addr: default_listen_addr(),
            port: default_port(),
            solana_rpc_url: default_solana_rpc_url(),
            pot_program_id: String::new(),
            difficulty: default_difficulty(),
            max_tensor_dim: default_max_tensor_dim(),
            max_mine_iterations: default_max_mine_iterations(),
            peer_network_mode: default_peer_network_mode(),
            pool_strategy: default_pool_strategy(),
            chain_bridge: default_chain_bridge(),
            device_protocol: default_device_protocol(),
            relayer_keypair_path: default_relayer_keypair_path(),
            auto_register_miners: default_auto_register_miners(),
            bootstrap_urls: default_bootstrap_urls(),
            enable_mdns: default_enable_mdns(),
            mdns_service_name: default_mdns_service_name(),
            internal_api_port: default_internal_api_port(),
            peer_timeout_secs: default_peer_timeout_secs(),
            challenge_relay_enabled: default_challenge_relay_enabled(),
            primary_validator_url: default_primary_validator_url(),
            maturity_depth: default_maturity_depth(),
            symmetry_num: default_symmetry_num(),
            symmetry_den: default_symmetry_den(),
            base_target: default_base_target(),
            protocol_fee_address: default_protocol_fee_address(),
            marketplace_fee_bps: default_marketplace_fee_bps(),
            tribe_mint_keypair_path: default_tribe_mint_keypair_path(),
            tribechain_enabled: default_tribechain_enabled(),
            tribechain_min_fee: default_tribechain_min_fee(),
            tribechain_max_pool_size: default_tribechain_max_pool_size(),
            tribechain_max_txs_per_block: default_tribechain_max_txs_per_block(),
            tribechain_genesis_path: default_tribechain_genesis_path(),
            tribechain_miner_address: default_tribechain_miner_address(),
            tribechain_blockstore_path: default_tribechain_blockstore_path(),
        }
    }
}
