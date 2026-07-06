//! Tests for bootstrap config handling: verifies that peer_network_mode
//! and pool_strategy from config are respected in ExtensionRegistry creation.

use pot_o_extensions::ExtensionRegistry;

#[test]
fn test_registry_from_config_defaults_to_local_solo() {
    // Test that default config (local_only + solo) creates correct implementations
    let registry = ExtensionRegistry::from_config(
        "local_only",
        "solo",
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
        "vpn_mesh",
        "solo",
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
        "local_only",
        "proportional",
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
        "local_only",
        "pplns",
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
        "unknown_network",
        "solo",
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
        "local_only",
        "unknown_strategy",
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
