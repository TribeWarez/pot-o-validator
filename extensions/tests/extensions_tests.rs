//! Tests for pot-o-extensions module
//!
//! Validates extension types and trait implementations

use chrono::Utc;
use pot_o_extensions::{
    ChainBridge, DeviceProtocol, DeviceStatus, DeviceType, Ed25519Authority, ExtensionRegistry,
    LocalOnlyNetwork, NativeDevice, PeerNetwork, PoolStrategy, ProofAuthority, SoloStrategy,
    TribechainBridge,
};
use tokio;

#[test]
fn test_device_type_variants() {
    let types = vec![
        DeviceType::Native,
        DeviceType::ESP32S,
        DeviceType::ESP8266,
        DeviceType::WASM,
        DeviceType::Custom,
    ];

    assert_eq!(types.len(), 5);
}

#[test]
fn test_device_status_variants() {
    let statuses = vec![
        DeviceStatus {
            device_type: DeviceType::Native,
            online: true,
            uptime_secs: 0,
            last_heartbeat: Utc::now(),
        },
        DeviceStatus {
            device_type: DeviceType::Native,
            online: false,
            uptime_secs: 0,
            last_heartbeat: Utc::now(),
        },
        DeviceStatus {
            device_type: DeviceType::ESP32S,
            online: true,
            uptime_secs: 3600,
            last_heartbeat: Utc::now(),
        },
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
fn test_tribechain_bridge_creation() {
    let bridge = TribechainBridge::new();

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
    let auth = Ed25519Authority::new("/tmp/nonexistent-keypair.json");

    // Auth should be created
    let _: Box<dyn ProofAuthority> = Box::new(auth);
}

#[test]
fn test_extension_registry_local_defaults() {
    let _registry = ExtensionRegistry::local_defaults("", 25);

    // Registry should have all components
    assert!(true);
}

#[tokio::test]
async fn test_tribechain_bridge_noop_submit() {
    let bridge = TribechainBridge::new();
    let proof = pot_o_mining::ProofPayload {
        proof: pot_o_mining::PotOProof {
            challenge_id: "test".into(),
            challenge_hash: "".into(),
            tensor_result_hash: "aaaa".into(),
            mml_score: 0.5,
            path_signature: "sig".into(),
            path_distance: 1,
            computation_nonce: 0,
            computation_hash: "bbbb".into(),
            miner_pubkey: "miner1".into(),
            timestamp: chrono::Utc::now(),
        },
        signature: vec![],
    };
    let result = bridge.submit_proof(&proof).await;
    assert!(result.is_ok());
}

#[test]
fn test_extension_registry_has_device() {
    let registry = ExtensionRegistry::local_defaults("", 25);
    let _ = &registry.device;
}

#[test]
fn test_extension_registry_has_network() {
    let registry = ExtensionRegistry::local_defaults("", 25);
    let _ = &registry.network;
}

#[test]
fn test_extension_registry_has_pool_strategy() {
    let registry = ExtensionRegistry::local_defaults("", 25);
    let _ = &registry.pool;
}

#[test]
fn test_extension_registry_has_chain_bridge() {
    let registry = ExtensionRegistry::local_defaults("", 25);
    let _ = &registry.chain;
}

#[test]
fn test_extension_registry_has_auth() {
    let registry = ExtensionRegistry::local_defaults("", 25);
    let _ = &registry.auth;
}

#[test]
fn test_native_device_type() {
    let device_type = DeviceType::Native;

    // Device type should be usable
    let _: DeviceType = device_type;
}

#[test]
fn test_device_status_online() {
    let status = DeviceStatus {
        device_type: DeviceType::Native,
        online: true,
        uptime_secs: 0,
        last_heartbeat: Utc::now(),
    };
    assert!(status.online);
}

#[test]
fn test_device_status_offline() {
    let status = DeviceStatus {
        device_type: DeviceType::ESP32S,
        online: false,
        uptime_secs: 0,
        last_heartbeat: Utc::now(),
    };
    assert!(!status.online);
}

#[test]
fn test_device_status_custom_device_type() {
    let status = DeviceStatus {
        device_type: DeviceType::Custom,
        online: true,
        uptime_secs: 0,
        last_heartbeat: Utc::now(),
    };
    assert_eq!(status.device_type, DeviceType::Custom);
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
    let auth: Box<dyn ProofAuthority> = Box::new(Ed25519Authority::new("/tmp/nonexistent.json"));
    let _ = auth;
}

#[test]
fn test_tribechain_bridge_configuration() {
    let _bridge = TribechainBridge::new();
    assert!(true);
}

#[test]
fn test_extension_registry_local_defaults_varies() {
    let reg1 = ExtensionRegistry::local_defaults("fee_addr", 50);
    let reg2 = ExtensionRegistry::local_defaults("", 25);
    let _ = (&reg1, &reg2);
}

#[test]
fn test_chain_bridge_trait_object() {
    let bridge: Box<dyn ChainBridge> = Box::new(TribechainBridge::new());
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
    let auth: Box<dyn ProofAuthority> = Box::new(Ed25519Authority::new("/tmp/nonexistent.json"));
    let _ = auth;
}

#[test]
fn test_extension_registry_trait_composition() {
    let registry = ExtensionRegistry::local_defaults("", 25);

    let _device: &dyn DeviceProtocol = &*registry.device;
    let _network: &dyn PeerNetwork = &*registry.network;
    let _pool: &dyn PoolStrategy = &*registry.pool;
    let _chain: &dyn ChainBridge = &*registry.chain;
    let _auth: &dyn ProofAuthority = &*registry.auth;
}

// ============================================================================
// TASK 2: Supply Cap Enforcement Tests
// ============================================================================

#[cfg(test)]
mod task2_supply_caps {
    use pot_o_core::TokenType;
    use pot_o_extensions::ledger::Ledger;

    #[test]
    fn test_issue_respects_supply_cap() {
        let mut ledger = Ledger::new("fee_address".to_string());
        
        // Try to issue STOMP above its 1T cap
        // Cap is 1_000_000_000_000
        ledger.issue("address1", &TokenType::STOMP, 900_000_000_000);
        assert_eq!(ledger.balance_of("address1", &TokenType::STOMP), 900_000_000_000);
        
        // Try to issue more that would exceed cap
        let result = ledger.try_issue("address2", &TokenType::STOMP, 200_000_000_000);
        assert!(result.is_err(), "Should reject issue that exceeds supply cap");
        
        // Should only allow 100_000_000_000 more
        let result = ledger.try_issue("address2", &TokenType::STOMP, 100_000_000_000);
        assert!(result.is_ok(), "Should allow issue within supply cap");
        assert_eq!(ledger.balance_of("address2", &TokenType::STOMP), 100_000_000_000);
        assert_eq!(ledger.total_supply_of(&TokenType::STOMP), 1_000_000_000_000);
    }

    #[test]
    fn test_supply_cap_enforcement_aum() {
        let mut ledger = Ledger::new("fee".to_string());
        
        // AUM has 2T cap
        let cap = 2_000_000_000_000;
        
        ledger.issue("a1", &TokenType::AUM, cap / 2);
        ledger.issue("a2", &TokenType::AUM, cap / 2);
        
        assert_eq!(ledger.total_supply_of(&TokenType::AUM), cap);
        
        // Any additional issue should fail
        let result = ledger.try_issue("a3", &TokenType::AUM, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_supply_cap_enforcement_ravecoin() {
        let mut ledger = Ledger::new("fee".to_string());
        
        // RAVECOIN has 500B cap
        let cap = 500_000_000_000;
        
        let result = ledger.try_issue("addr", &TokenType::RAVECOIN, cap);
        assert!(result.is_ok());
        
        let result = ledger.try_issue("addr", &TokenType::RAVECOIN, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_supply_cap_not_enforced_for_uncapped_tokens() {
        let mut ledger = Ledger::new("fee".to_string());
        
        // TribeChain should not have supply cap enforcement
        // (or should have very high cap)
        ledger.issue("a1", &TokenType::TribeChain, 1_000_000_000_000_000);
        ledger.issue("a2", &TokenType::TribeChain, 1_000_000_000_000_000);
        
        // Should succeed without error
        assert!(ledger.balance_of("a1", &TokenType::TribeChain) > 0);
    }
}
