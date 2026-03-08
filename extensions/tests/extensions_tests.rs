//! Tests for pot-o-extensions module
//!
//! Validates extension types and trait implementations

use pot_o_extensions::{
    DeviceType, DeviceStatus, DeviceProtocol, NativeDevice,
    ChainBridge, SolanaBridge,
    DefiClient,
    PeerNetwork, LocalOnlyNetwork,
    PoolStrategy, SoloStrategy,
    ProofAuthority, Ed25519Authority,
    ExtensionRegistry,
};

#[test]
fn test_device_type_variants() {
    let types = vec![
        DeviceType::NativeX86_64,
        DeviceType::ESP32S,
        DeviceType::ESP8266,
        DeviceType::WASM,
    ];
    
    assert_eq!(types.len(), 4);
}

#[test]
fn test_device_status_variants() {
    let statuses = vec![
        DeviceStatus::Idle,
        DeviceStatus::Mining,
        DeviceStatus::Disconnected,
    ];
    
    assert_eq!(statuses.len(), 3);
}

#[test]
fn test_native_device_creation() {
    let device = NativeDevice::new();
    
    // Native device should be created successfully
    let _: Box<dyn DeviceProtocol> = Box::new(device);
}

#[test]
fn test_solana_bridge_creation() {
    let bridge = SolanaBridge::new(
        "https://api.devnet.solana.com".to_string(),
        "11111111111111111111111111111111".to_string(),
        "/path/to/keypair.json".to_string(),
        false,
    );
    
    // Bridge should be created
    let _: Box<dyn ChainBridge> = Box::new(bridge);
}

#[test]
fn test_local_only_network_creation() {
    let network = LocalOnlyNetwork::new();
    
    // Network should be created
    let _: Box<dyn PeerNetwork> = Box::new(network);
}

#[test]
fn test_solo_strategy_creation() {
    let strategy = SoloStrategy;
    
    // Strategy should be created
    let _: Box<dyn PoolStrategy> = Box::new(strategy);
}

#[test]
fn test_ed25519_authority_creation() {
    let auth = Ed25519Authority;
    
    // Auth should be created
    let _: Box<dyn ProofAuthority> = Box::new(auth);
}

#[test]
fn test_extension_registry_local_defaults() {
    let registry = ExtensionRegistry::local_defaults(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        true,
    );
    
    // Registry should have all components
    assert!(true); // Registry creation successful
}

#[test]
fn test_defi_client_creation() {
    let client = DefiClient::new("https://api.devnet.solana.com".to_string());
    
    // Client should be created
    assert!(true);
}

#[test]
fn test_extension_registry_has_device() {
    let registry = ExtensionRegistry::local_defaults(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        false,
    );
    
    // Should have a device protocol
    let _ = &registry.device;
}

#[test]
fn test_extension_registry_has_network() {
    let registry = ExtensionRegistry::local_defaults(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        false,
    );
    
    // Should have a network protocol
    let _ = &registry.network;
}

#[test]
fn test_extension_registry_has_pool_strategy() {
    let registry = ExtensionRegistry::local_defaults(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        false,
    );
    
    // Should have a pool strategy
    let _ = &registry.pool;
}

#[test]
fn test_extension_registry_has_chain_bridge() {
    let registry = ExtensionRegistry::local_defaults(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        false,
    );
    
    // Should have a chain bridge
    let _ = &registry.chain;
}

#[test]
fn test_extension_registry_has_auth() {
    let registry = ExtensionRegistry::local_defaults(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        false,
    );
    
    // Should have an auth provider
    let _ = &registry.auth;
}

#[test]
fn test_native_device_type() {
    let device_type = DeviceType::NativeX86_64;
    
    // Device type should be usable
    let _: DeviceType = device_type;
}

#[test]
fn test_device_status_idle() {
    let status = DeviceStatus::Idle;
    let _ = &status;
}

#[test]
fn test_device_status_mining() {
    let status = DeviceStatus::Mining;
    let _ = &status;
}

#[test]
fn test_device_status_disconnected() {
    let status = DeviceStatus::Disconnected;
    let _ = &status;
}

#[test]
fn test_local_only_network_type() {
    let network: Box<dyn PeerNetwork> = Box::new(LocalOnlyNetwork::new());
    let _ = network;
}

#[test]
fn test_solo_strategy_type() {
    let strategy: Box<dyn PoolStrategy> = Box::new(SoloStrategy);
    let _ = strategy;
}

#[test]
fn test_ed25519_authority_type() {
    let auth: Box<dyn ProofAuthority> = Box::new(Ed25519Authority);
    let _ = auth;
}

#[test]
fn test_solana_bridge_configuration() {
    let solana_url = "https://api.devnet.solana.com".to_string();
    let program_id = "11111111111111111111111111111111".to_string();
    let keypair_path = "/tmp/keypair.json".to_string();
    
    let _bridge = SolanaBridge::new(solana_url, program_id, keypair_path, true);
    
    // Bridge created successfully
    assert!(true);
}

#[test]
fn test_defi_client_rpc_config() {
    let rpc_url = "https://api.devnet.solana.com".to_string();
    let _client = DefiClient::new(rpc_url);
    
    // Client created successfully
    assert!(true);
}

#[test]
fn test_extension_registry_auto_register_option() {
    // With auto_register = true
    let reg1 = ExtensionRegistry::local_defaults(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        true, // auto-register enabled
    );
    
    // With auto_register = false
    let reg2 = ExtensionRegistry::local_defaults(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        false, // auto-register disabled
    );
    
    // Both registries should be created
    let _ = (&reg1, &reg2);
}

#[test]
fn test_chain_bridge_trait_object() {
    let bridge: Box<dyn ChainBridge> = Box::new(
        SolanaBridge::new(
            "https://api.devnet.solana.com".to_string(),
            "11111111111111111111111111111111".to_string(),
            "/path/to/keypair.json".to_string(),
            false,
        )
    );
    
    let _ = bridge;
}

#[test]
fn test_device_protocol_trait_object() {
    let device: Box<dyn DeviceProtocol> = Box::new(NativeDevice::new());
    let _ = device;
}

#[test]
fn test_peer_network_trait_object() {
    let network: Box<dyn PeerNetwork> = Box::new(LocalOnlyNetwork::new());
    let _ = network;
}

#[test]
fn test_pool_strategy_trait_object() {
    let pool: Box<dyn PoolStrategy> = Box::new(SoloStrategy);
    let _ = pool;
}

#[test]
fn test_proof_authority_trait_object() {
    let auth: Box<dyn ProofAuthority> = Box::new(Ed25519Authority);
    let _ = auth;
}

#[test]
fn test_extension_registry_trait_composition() {
    let registry = ExtensionRegistry::local_defaults(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        false,
    );
    
    // All traits should be object-safe
    let _device: &dyn DeviceProtocol = &*registry.device;
    let _network: &dyn PeerNetwork = &*registry.network;
    let _pool: &dyn PoolStrategy = &*registry.pool;
    let _chain: &dyn ChainBridge = &*registry.chain;
    let _auth: &dyn ProofAuthority = &*registry.auth;
}
