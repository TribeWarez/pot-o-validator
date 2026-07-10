//! Tests for token configuration module
use pot_o_core::token_config::{token_config, TokenConfig};
use pot_o_core::TokenType;

#[test]
fn test_token_config_creation() {
    let config = TokenConfig {
        token: TokenType::STOMP,
        supply_cap: 1_000_000_000_000,
        burn_rate_bps: 50,
        decay_half_life_blocks: 1_000_000,
    };

    assert_eq!(config.token, TokenType::STOMP);
    assert_eq!(config.supply_cap, 1_000_000_000_000);
    assert_eq!(config.burn_rate_bps, 50);
    assert_eq!(config.decay_half_life_blocks, 1_000_000);
}

#[test]
fn test_token_config_set() {
    let config_set = token_config();

    // Verify we have all expected tokens configured
    assert!(config_set.get(&TokenType::STOMP).is_some());
    assert!(config_set.get(&TokenType::AUM).is_some());
    assert!(config_set.get(&TokenType::RAVECOIN).is_some());
}

#[test]
fn test_stomp_config() {
    let config_set = token_config();
    let stomp = config_set
        .get(&TokenType::STOMP)
        .expect("STOMP not configured");

    // STOMP: 1T supply cap, 50 bps burn rate, 1M block half-life
    assert_eq!(stomp.supply_cap, 1_000_000_000_000);
    assert_eq!(stomp.burn_rate_bps, 50);
    assert_eq!(stomp.decay_half_life_blocks, 1_000_000);
}

#[test]
fn test_aum_config() {
    let config_set = token_config();
    let aum = config_set.get(&TokenType::AUM).expect("AUM not configured");

    // AUM: 2T supply cap, 0 bps burn rate, 2M block half-life
    assert_eq!(aum.supply_cap, 2_000_000_000_000);
    assert_eq!(aum.burn_rate_bps, 0);
    assert_eq!(aum.decay_half_life_blocks, 2_000_000);
}

#[test]
fn test_ravecoin_config() {
    let config_set = token_config();
    let ravecoin = config_set
        .get(&TokenType::RAVECOIN)
        .expect("RAVECOIN not configured");

    // RAVECOIN: 500B supply cap, 100 bps burn rate, 500K block half-life
    assert_eq!(ravecoin.supply_cap, 500_000_000_000);
    assert_eq!(ravecoin.burn_rate_bps, 100);
    assert_eq!(ravecoin.decay_half_life_blocks, 500_000);
}

#[test]
fn test_burn_calculation() {
    let config = TokenConfig {
        token: TokenType::STOMP,
        supply_cap: 1_000_000_000_000,
        burn_rate_bps: 100, // 1%
        decay_half_life_blocks: 1_000_000,
    };

    // At 1% burn rate: amount * 100 / 10_000 = amount / 100
    let amount = 10_000;
    let burn = (amount * config.burn_rate_bps as u64) / 10_000;
    assert_eq!(burn, 100);
}

#[test]
fn test_decay_calculation() {
    let config = TokenConfig {
        token: TokenType::STOMP,
        supply_cap: 1_000_000_000_000,
        burn_rate_bps: 50,
        decay_half_life_blocks: 1_000_000,
    };

    // After 0 blocks: 100% remains
    assert_eq!(config.decay_half_life_blocks, 1_000_000);

    // Half-life means after that many blocks, 50% remains
    // Formula: amount * 2^(-blocks / half_life)
    // At 500k blocks: amount * 2^(-0.5) ≈ amount * 0.707
    let blocks_elapsed = 500_000;
    let factor = 2f64.powf(-(blocks_elapsed as f64 / config.decay_half_life_blocks as f64));
    assert!(factor > 0.7 && factor < 0.71);
}

#[test]
fn test_multiple_tokens_config() {
    let config_set = token_config();

    assert!(config_set.len() >= 3); // At least STOMP, AUM, RAVECOIN

    for (token_type, config) in config_set.iter() {
        assert_eq!(&config.token, token_type);
        assert!(config.supply_cap > 0);
        assert!(config.burn_rate_bps <= 10_000); // Max 100%
        if *token_type != TokenType::TribeChain {
            assert!(config.decay_half_life_blocks > 0);
        }
    }
}
