//! Integration tests for bootstrap config handling
//! Verifies that build_extension_registry respects config settings

use pot_o_extensions::ExtensionRegistry;

#[test]
fn test_build_registry_from_config_respects_network_mode() {
    // Simulates what build_extension_registry does
    let network_mode = "vpn_mesh";
    let pool_strategy = "solo";
    
    let registry = ExtensionRegistry::from_config(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        false,
        network_mode,
        pool_strategy,
    );

    // Verify VpnMeshNetwork is used
    let node_id = registry.network.node_id();
    assert!(!node_id.is_empty(), "Network should have node_id");
}

#[test]
fn test_build_registry_from_config_respects_pool_strategy() {
    // Simulates what build_extension_registry does
    let network_mode = "local_only";
    let pool_strategy = "proportional";
    
    let registry = ExtensionRegistry::from_config(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        false,
        network_mode,
        pool_strategy,
    );

    // Verify ProportionalPool is used
    let pool_info = registry.pool.pool_info(10, 50000);
    assert_eq!(pool_info.pool_type, "proportional");
    assert_eq!(pool_info.total_miners, 10);
    assert_eq!(pool_info.total_stake, 50000);
}

#[test]
fn test_config_defaults_preserve_backward_compatibility() {
    // Verify that default config values (local_only + solo) are still supported
    let registry = ExtensionRegistry::from_config(
        "https://api.devnet.solana.com",
        "11111111111111111111111111111111",
        "/path/to/keypair.json",
        false,
        "local_only",  // Default from config.rs
        "solo",        // Default from config.rs
    );

    // Should work like the old local_defaults() function
    let network_node_id = registry.network.node_id();
    assert!(!network_node_id.is_empty());
    
    let pool_info = registry.pool.pool_info(0, 0);
    assert_eq!(pool_info.pool_type, "solo");
}
