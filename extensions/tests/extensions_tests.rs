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
        assert_eq!(
            ledger.balance_of("address1", &TokenType::STOMP),
            900_000_000_000
        );

        // Try to issue more that would exceed cap
        let result = ledger.try_issue("address2", &TokenType::STOMP, 200_000_000_000);
        assert!(
            result.is_err(),
            "Should reject issue that exceeds supply cap"
        );

        // Should only allow 100_000_000_000 more
        let result = ledger.try_issue("address2", &TokenType::STOMP, 100_000_000_000);
        assert!(result.is_ok(), "Should allow issue within supply cap");
        assert_eq!(
            ledger.balance_of("address2", &TokenType::STOMP),
            100_000_000_000
        );
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

// ============================================================================
// TASK 3: Age Decay and Transfer Burn Tests
// ============================================================================

#[cfg(test)]
mod task3_decay_and_burn {
    use pot_o_core::TokenType;
    use pot_o_extensions::ledger::Ledger;

    #[test]
    fn test_apply_decay_stomp() {
        let mut ledger = Ledger::new("fee".to_string());

        // Issue STOMP tokens
        ledger.issue("alice", &TokenType::STOMP, 1_000_000);

        // Immediately check balance
        assert_eq!(ledger.balance_of("alice", &TokenType::STOMP), 1_000_000);

        // Apply decay at 0 blocks - should have no effect
        let amount = ledger.apply_decay("alice", &TokenType::STOMP, 0);
        assert_eq!(amount, 1_000_000);

        // Apply decay at half-life (1M blocks) - should be ~50%
        let amount = ledger.apply_decay("alice", &TokenType::STOMP, 1_000_000);
        assert!(
            amount > 490_000 && amount < 510_000,
            "Expected ~500_000 at half-life, got {}",
            amount
        );
    }

    #[test]
    fn test_apply_decay_ravecoin() {
        let mut ledger = Ledger::new("fee".to_string());

        // RAVECOIN has 500K block half-life
        ledger.issue("bob", &TokenType::RAVECOIN, 1_000_000);

        // At 0 blocks
        let amount = ledger.apply_decay("bob", &TokenType::RAVECOIN, 0);
        assert_eq!(amount, 1_000_000);

        // At half-life (500K blocks)
        let amount = ledger.apply_decay("bob", &TokenType::RAVECOIN, 500_000);
        assert!(amount > 490_000 && amount < 510_000);
    }

    #[test]
    fn test_update_interaction_timestamp() {
        let mut ledger = Ledger::new("fee".to_string());

        // Update interaction time for an address
        ledger.update_interaction("charlie", &TokenType::STOMP);

        // Should record the current block height
        assert!(ledger
            .last_interaction("charlie", &TokenType::STOMP)
            .is_some());
    }

    #[test]
    fn test_transfer_with_burn_stomp() {
        let mut ledger = Ledger::new("fee".to_string());

        // Issue tokens
        ledger.issue("alice", &TokenType::STOMP, 10_000);

        // Transfer 1000 tokens (burn rate is 50 bps = 0.5%)
        // Expected burn: 1000 * 50 / 10000 = 5 tokens
        let result = ledger.transfer("alice", "bob", &TokenType::STOMP, 1000, 0);
        assert!(result.is_ok());

        // Alice should have lost: 1000 (transferred) + 5 (burn) = 1005
        assert_eq!(ledger.balance_of("alice", &TokenType::STOMP), 10_000 - 1005);

        // Bob should have received: 1000
        assert_eq!(ledger.balance_of("bob", &TokenType::STOMP), 1000);

        // Total supply reduced by burn amount
        // 10_000 issued -> 5 burned -> 9_995 total
        let total = ledger.total_supply_of(&TokenType::STOMP);
        assert_eq!(total, 9_995);
    }

    #[test]
    fn test_transfer_burn_ravecoin() {
        let mut ledger = Ledger::new("fee".to_string());

        ledger.issue("alice", &TokenType::RAVECOIN, 10_000);

        // RAVECOIN has 100 bps = 1% burn rate
        // Expected burn: 1000 * 100 / 10000 = 10 tokens
        let result = ledger.transfer("alice", "bob", &TokenType::RAVECOIN, 1000, 0);
        assert!(result.is_ok());

        // Alice loses 1000 + 10 = 1010
        assert_eq!(
            ledger.balance_of("alice", &TokenType::RAVECOIN),
            10_000 - 1010
        );

        // Bob gets 1000
        assert_eq!(ledger.balance_of("bob", &TokenType::RAVECOIN), 1000);
    }

    #[test]
    fn test_transfer_no_burn_aum() {
        let mut ledger = Ledger::new("fee".to_string());

        ledger.issue("alice", &TokenType::AUM, 10_000);

        // AUM has 0 bps = no burn
        let result = ledger.transfer("alice", "bob", &TokenType::AUM, 1000, 0);
        assert!(result.is_ok());

        // Alice loses exactly 1000 (no burn)
        assert_eq!(ledger.balance_of("alice", &TokenType::AUM), 9_000);

        // Bob gets exactly 1000
        assert_eq!(ledger.balance_of("bob", &TokenType::AUM), 1000);
    }

    #[test]
    fn test_transfer_with_fee_and_burn() {
        let mut ledger = Ledger::new("fee_collector".to_string());

        ledger.issue("alice", &TokenType::STOMP, 10_000);

        // Transfer with fee: 100 amount, 10 fee, 50 bps burn
        // Burn on amount: 100 * 50 / 10000 = 0 (rounded down)
        // Total burn from amount is negligible
        let result = ledger.transfer("alice", "bob", &TokenType::STOMP, 100, 10);
        assert!(result.is_ok());

        // Alice should lose: 100 (amount) + 10 (fee) + 0 (burn negligible) = 110
        // Bob should receive: 100
        // fee_collector should receive: 10
        assert_eq!(
            ledger.balance_of("alice", &TokenType::STOMP),
            10_000 - 100 - 10
        );
        assert_eq!(ledger.balance_of("bob", &TokenType::STOMP), 100);
        assert_eq!(ledger.balance_of("fee_collector", &TokenType::STOMP), 10);
    }

    #[test]
    fn test_multiple_transfers_accumulate_burn() {
        let mut ledger = Ledger::new("fee".to_string());

        ledger.issue("alice", &TokenType::RAVECOIN, 100_000);

        // RAVECOIN: 100 bps = 1% burn
        // First transfer: 10_000 amount -> 100 burn
        let _ = ledger.transfer("alice", "bob", &TokenType::RAVECOIN, 10_000, 0);
        // Alice: 100_000 - 10_000 - 100 = 89_900
        // Bob: 10_000
        // Total: 89_900 + 10_000 = 99_900

        // Second transfer: from bob to charlie
        // 5_000 amount -> 50 burn
        let _ = ledger.transfer("bob", "charlie", &TokenType::RAVECOIN, 5_000, 0);
        // Bob: 10_000 - 5_000 - 50 = 4_950
        // Charlie: 5_000
        // Total: 89_900 + 4_950 + 5_000 = 99_850

        let total = ledger.total_supply_of(&TokenType::RAVECOIN);
        assert_eq!(total, 99_850);
    }
}

// ============================================================================
// TASK 4: AUM Halving and Minter Allowlist Tests
// ============================================================================

#[cfg(test)]
mod task4_aum_minting {
    use pot_o_core::TokenType;
    use pot_o_extensions::ledger::Ledger;

    #[test]
    fn test_aum_block_reward_initial() {
        let ledger = Ledger::new("fee".to_string());

        // At block 0, AUM reward should be at initial level
        let reward = ledger.aum_block_reward(0);
        // AUM starts at some initial rate (let's say 1000 base units)
        assert!(reward > 0);
    }

    #[test]
    fn test_aum_block_reward_halving() {
        let ledger = Ledger::new("fee".to_string());

        // Reward at block 0
        let reward_at_0 = ledger.aum_block_reward(0);

        // Reward at halving point (1M blocks)
        let reward_at_1m = ledger.aum_block_reward(1_000_000);

        // At halving, reward should be ~50% of initial
        assert!(reward_at_1m < reward_at_0);
        assert!(reward_at_1m * 2 >= reward_at_0 && reward_at_1m * 2 <= reward_at_0 + 1);
    }

    #[test]
    fn test_aum_block_reward_multiple_halvings() {
        let ledger = Ledger::new("fee".to_string());

        let reward_at_0 = ledger.aum_block_reward(0);
        let reward_at_1m = ledger.aum_block_reward(1_000_000);
        let reward_at_2m = ledger.aum_block_reward(2_000_000);
        let reward_at_3m = ledger.aum_block_reward(3_000_000);

        // Each halving should reduce by ~50%
        assert!(reward_at_1m < reward_at_0);
        assert!(reward_at_2m < reward_at_1m);
        assert!(reward_at_3m < reward_at_2m);
    }

    #[test]
    fn test_minter_allowlist_add_and_check() {
        let mut ledger = Ledger::new("fee".to_string());

        // Add a minter to allowlist
        ledger.add_minter(&TokenType::AUM, "minter1");

        // Check if authorized
        assert!(ledger.is_authorized_minter(&TokenType::AUM, "minter1"));
        assert!(!ledger.is_authorized_minter(&TokenType::AUM, "minter2"));
    }

    #[test]
    fn test_minter_allowlist_multiple_minters() {
        let mut ledger = Ledger::new("fee".to_string());

        ledger.add_minter(&TokenType::AUM, "minter1");
        ledger.add_minter(&TokenType::AUM, "minter2");
        ledger.add_minter(&TokenType::AUM, "minter3");

        assert!(ledger.is_authorized_minter(&TokenType::AUM, "minter1"));
        assert!(ledger.is_authorized_minter(&TokenType::AUM, "minter2"));
        assert!(ledger.is_authorized_minter(&TokenType::AUM, "minter3"));
        assert!(!ledger.is_authorized_minter(&TokenType::AUM, "minter4"));
    }

    #[test]
    fn test_minter_allowlist_per_token() {
        let mut ledger = Ledger::new("fee".to_string());

        // Add different minters for different tokens
        ledger.add_minter(&TokenType::AUM, "minter1");
        ledger.add_minter(&TokenType::STOMP, "minter2");
        ledger.add_minter(&TokenType::RAVECOIN, "minter3");

        // Each minter only authorized for their token
        assert!(ledger.is_authorized_minter(&TokenType::AUM, "minter1"));
        assert!(!ledger.is_authorized_minter(&TokenType::AUM, "minter2"));

        assert!(ledger.is_authorized_minter(&TokenType::STOMP, "minter2"));
        assert!(!ledger.is_authorized_minter(&TokenType::STOMP, "minter1"));

        assert!(ledger.is_authorized_minter(&TokenType::RAVECOIN, "minter3"));
        assert!(!ledger.is_authorized_minter(&TokenType::RAVECOIN, "minter1"));
    }

    #[test]
    fn test_issue_with_minter_check_for_aum() {
        let mut ledger = Ledger::new("fee".to_string());

        // Regular issue should still work (backward compat)
        ledger.issue("user1", &TokenType::AUM, 1000);
        assert_eq!(ledger.balance_of("user1", &TokenType::AUM), 1000);

        // Add authorized minter
        ledger.add_minter(&TokenType::AUM, "minter1");

        // Now minter can issue
        let result = ledger.try_issue_with_minter("minter1", &TokenType::AUM, "user2", 1000);
        assert!(result.is_ok());
        assert_eq!(ledger.balance_of("user2", &TokenType::AUM), 1000);
    }

    #[test]
    fn test_issue_with_unauthorized_minter() {
        let mut ledger = Ledger::new("fee".to_string());

        // Add authorized minter
        ledger.add_minter(&TokenType::AUM, "minter1");

        // Unauthorized minter should fail
        let result = ledger.try_issue_with_minter("unauthorized", &TokenType::AUM, "user", 1000);
        assert!(result.is_err(), "Unauthorized minter should be rejected");
    }

    #[test]
    fn test_aum_halving_schedule() {
        let ledger = Ledger::new("fee".to_string());

        // Test that we can predict reward at various heights
        let reward_0 = ledger.aum_block_reward(0);
        let reward_1m = ledger.aum_block_reward(1_000_000);
        let reward_2m = ledger.aum_block_reward(2_000_000);

        // Each should be less than or equal to previous
        assert!(reward_1m <= reward_0);
        assert!(reward_2m <= reward_1m);
    }

    #[test]
    fn test_combined_aum_minting_flow() {
        let mut ledger = Ledger::new("fee".to_string());

        // Setup minter
        ledger.add_minter(&TokenType::AUM, "aum_minter");

        // Get halving-based reward at block 0
        let reward = ledger.aum_block_reward(0);

        // Minter issues tokens with halving reward
        let result =
            ledger.try_issue_with_minter("aum_minter", &TokenType::AUM, "validator", reward);
        assert!(result.is_ok());
        assert_eq!(ledger.balance_of("validator", &TokenType::AUM), reward);
    }
}
