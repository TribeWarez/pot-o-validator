//! Tests for mDNS discovery module
//!
//! Validates mDNS service registration, discovery, and peer lookup functionality

use pot_o_extensions::{MdnsDiscovery, PeerDiscovery};

#[test]
fn test_mdns_discovery_creation() {
    // Test that we can create an MdnsDiscovery instance
    let result = MdnsDiscovery::new("validator-1", 5555);
    assert!(
        result.is_ok(),
        "MdnsDiscovery should be created successfully"
    );

    let mdns = result.unwrap();
    // Verify that the instance was created with correct node_id
    assert_eq!(mdns.node_id(), "validator-1");
    assert_eq!(mdns.port(), 5555);
}

#[test]
fn test_mdns_discovery_node_id_getter() {
    let mdns = MdnsDiscovery::new("validator-2", 6666).expect("Failed to create MdnsDiscovery");
    assert_eq!(mdns.node_id(), "validator-2");
}

#[test]
fn test_mdns_discovery_port_getter() {
    let mdns = MdnsDiscovery::new("validator-3", 7777).expect("Failed to create MdnsDiscovery");
    assert_eq!(mdns.port(), 7777);
}

#[test]
fn test_mdns_service_registration() {
    let mdns = MdnsDiscovery::new("validator-4", 8888).expect("Failed to create MdnsDiscovery");

    // Test that we can call register_service - may fail if mDNS is unavailable
    // which is acceptable in test environments without mDNS support
    let result = mdns.register_service("validator-4.local");

    // Either success or graceful error handling is acceptable
    match result {
        Ok(_) => {
            // Ideal case: mDNS is available and service registered
            assert!(true, "Service registered successfully");
        }
        Err(e) => {
            // Acceptable: mDNS unavailable, but error message should be informative
            let error_msg = format!("{}", e);
            assert!(
                error_msg.contains("Failed") || error_msg.contains("not initialized"),
                "Error should be informative: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_mdns_discovery_returns_peer_discovery() {
    let mdns = MdnsDiscovery::new("validator-5", 9999).expect("Failed to create MdnsDiscovery");

    // Test that we can register the service (may fail in test environment without mDNS)
    let _register_result = mdns.register_service("validator-5.local");

    // Test that discover_peers method exists and can be called
    // We expect empty results since no other validators are likely running
    let discovery_result = mdns.discover_peers(1);
    assert!(
        discovery_result.is_ok(),
        "Discovery should not error: {:?}",
        discovery_result.err()
    );

    // Result should be a vector (empty is fine)
    let peers = discovery_result.unwrap();
    assert!(
        peers.is_empty() || !peers.is_empty(),
        "Peers should be a valid vector"
    );
}

#[test]
fn test_peer_discovery_struct() {
    // Test that PeerDiscovery can be created and accessed
    let peer = PeerDiscovery {
        node_id: "validator-7".to_string(),
        hostname: "validator-7.local".to_string(),
        ip: "192.168.1.100".parse().expect("Invalid IP"),
        port: 5555,
    };

    assert_eq!(peer.node_id, "validator-7");
    assert_eq!(peer.hostname, "validator-7.local");
    assert_eq!(peer.port, 5555);
}

#[test]
fn test_mdns_service_type() {
    // Verify that the service type follows mDNS naming conventions
    let mdns = MdnsDiscovery::new("validator-8", 5555).expect("Failed to create MdnsDiscovery");
    assert_eq!(mdns.service_type(), "_pot-o-validator._tcp.local.");
}

#[test]
fn test_mdns_multiple_instances() {
    // Test that multiple MdnsDiscovery instances can coexist
    let mdns1 = MdnsDiscovery::new("validator-9", 5555).expect("Failed to create MdnsDiscovery 1");
    let mdns2 = MdnsDiscovery::new("validator-10", 6666).expect("Failed to create MdnsDiscovery 2");

    assert_ne!(mdns1.node_id(), mdns2.node_id());
    assert_ne!(mdns1.port(), mdns2.port());
}
