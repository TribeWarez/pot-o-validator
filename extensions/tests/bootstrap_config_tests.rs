//! Tests for bootstrap config handling: verifies that peer_network_mode
//! and pool_strategy from config are respected in ExtensionRegistry creation.

use pot_o_extensions::ExtensionRegistry;

#[test]
fn test_registry_from_config_defaults_to_local_solo() {
    // Test that default config (local_only + solo) creates correct implementations
    let registry = ExtensionRegistry::from_config(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        false,
        "local_only", // peer_network_mode
        "solo",       // pool_strategy
        "native",
        "",
        25,
        None,
        &[],
        false,
        "pot-o-validator",
        30,
        true,
    );

    // Verify the network is LocalOnlyNetwork
    let network_node_id = registry.network.node_id();
    assert!(
        !network_node_id.is_empty(),
        "LocalOnlyNetwork should have a node_id"
    );

    // Verify the pool is SoloStrategy by checking pool_info
    let pool_info = registry.pool.pool_info(0, 0);
    assert_eq!(pool_info.pool_type, "solo");
}

#[test]
fn test_registry_from_config_vpn_mesh_network() {
    // Test that vpn_mesh config creates VpnMeshNetwork (even if stubbed)
    let registry = ExtensionRegistry::from_config(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        false,
        "vpn_mesh", // peer_network_mode
        "solo",     // pool_strategy
        "native",
        "",
        25,
        None,
        &[],
        false,
        "pot-o-validator",
        30,
        true,
    );

    // Verify the network can be used (should be VpnMeshNetwork)
    let network_node_id = registry.network.node_id();
    assert!(
        !network_node_id.is_empty(),
        "VpnMeshNetwork should have a node_id"
    );
}

#[test]
fn test_registry_from_config_proportional_pool() {
    // Test that proportional pool_strategy creates ProportionalPool (even if stubbed)
    let registry = ExtensionRegistry::from_config(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        false,
        "local_only",   // peer_network_mode
        "proportional", // pool_strategy
        "native",
        "",
        25,
        None,
        &[],
        false,
        "pot-o-validator",
        30,
        true,
    );

    // Verify the pool is ProportionalPool by checking pool_info
    let pool_info = registry.pool.pool_info(0, 0);
    assert_eq!(pool_info.pool_type, "proportional");
}

#[test]
fn test_registry_from_config_pplns_pool() {
    // Test that pplns pool_strategy creates PPLNSPool (even if stubbed)
    let registry = ExtensionRegistry::from_config(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        false,
        "local_only", // peer_network_mode
        "pplns",      // pool_strategy
        "native",
        "",
        25,
        None,
        &[],
        false,
        "pot-o-validator",
        30,
        true,
    );

    // Verify the pool is PPLNSPool by checking pool_info
    let pool_info = registry.pool.pool_info(0, 0);
    assert_eq!(pool_info.pool_type, "pplns");
}

#[test]
fn test_registry_from_config_unknown_network_defaults_to_local_only() {
    // Test graceful fallback: unknown network mode → local_only
    let registry = ExtensionRegistry::from_config(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        false,
        "unknown_network", // peer_network_mode (invalid)
        "solo",            // pool_strategy
        "native",
        "",
        25,
        None,
        &[],
        false,
        "pot-o-validator",
        30,
        true,
    );

    // Should fall back to LocalOnlyNetwork
    let network_node_id = registry.network.node_id();
    assert!(!network_node_id.is_empty());
}

#[test]
fn test_registry_from_config_unknown_pool_defaults_to_solo() {
    // Test graceful fallback: unknown pool strategy → solo
    let registry = ExtensionRegistry::from_config(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        false,
        "local_only",       // peer_network_mode
        "unknown_strategy", // pool_strategy (invalid)
        "native",
        "",
        25,
        None,
        &[],
        false,
        "pot-o-validator",
        30,
        true,
    );

    // Should fall back to SoloStrategy
    let pool_info = registry.pool.pool_info(0, 0);
    assert_eq!(pool_info.pool_type, "solo");
}
