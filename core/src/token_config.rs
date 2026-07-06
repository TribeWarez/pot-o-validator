//! Token configuration module
//!
//! Defines per-token configuration including supply caps, burn rates, and decay half-lives.

use crate::TokenType;
use std::collections::HashMap;

/// Configuration for a single token type
#[derive(Debug, Clone)]
pub struct TokenConfig {
    /// Token type
    pub token: TokenType,
    /// Maximum supply cap (in smallest unit)
    pub supply_cap: u64,
    /// Burn rate in basis points (bps) per transfer
    /// E.g., 50 bps = 0.5% burn rate
    pub burn_rate_bps: u16,
    /// Blocks until token amount decays to 50% of original value
    pub decay_half_life_blocks: u64,
}

impl TokenConfig {
    /// Calculate burn amount based on transfer amount
    pub fn calculate_burn(&self, amount: u64) -> u64 {
        (amount as u128 * self.burn_rate_bps as u128 / 10_000) as u64
    }

    /// Calculate remaining amount after decay
    /// Uses formula: amount * 2^(-blocks / half_life)
    pub fn apply_decay(&self, amount: u64, blocks_elapsed: u64) -> u64 {
        if blocks_elapsed == 0 {
            return amount;
        }

        let decay_factor = 2f64.powf(-(blocks_elapsed as f64 / self.decay_half_life_blocks as f64));
        (amount as f64 * decay_factor) as u64
    }
}

/// Type alias for a set of token configurations
pub type TokenConfigSet = HashMap<TokenType, TokenConfig>;

/// Returns the global token configuration set
pub fn token_config() -> TokenConfigSet {
    let mut configs = HashMap::new();

    // STOMP: 1 trillion supply cap, 50 bps (0.5%) burn rate, 1M block half-life
    configs.insert(
        TokenType::STOMP,
        TokenConfig {
            token: TokenType::STOMP,
            supply_cap: 1_000_000_000_000,
            burn_rate_bps: 50,
            decay_half_life_blocks: 1_000_000,
        },
    );

    // AUM: 2 trillion supply cap, 0 bps burn rate (no burn), 2M block half-life
    configs.insert(
        TokenType::AUM,
        TokenConfig {
            token: TokenType::AUM,
            supply_cap: 2_000_000_000_000,
            burn_rate_bps: 0,
            decay_half_life_blocks: 2_000_000,
        },
    );

    // RAVECOIN: 500 billion supply cap, 100 bps (1%) burn rate, 500K block half-life
    configs.insert(
        TokenType::RAVECOIN,
        TokenConfig {
            token: TokenType::RAVECOIN,
            supply_cap: 500_000_000_000,
            burn_rate_bps: 100,
            decay_half_life_blocks: 500_000,
        },
    );

    configs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_burn() {
        let config = TokenConfig {
            token: TokenType::STOMP,
            supply_cap: 1_000_000_000_000,
            burn_rate_bps: 100, // 1%
            decay_half_life_blocks: 1_000_000,
        };

        let burn = config.calculate_burn(10_000);
        assert_eq!(burn, 100);
    }

    #[test]
    fn test_apply_decay() {
        let config = TokenConfig {
            token: TokenType::STOMP,
            supply_cap: 1_000_000_000_000,
            burn_rate_bps: 50,
            decay_half_life_blocks: 1_000_000,
        };

        // At 0 blocks, no decay
        assert_eq!(config.apply_decay(1_000_000, 0), 1_000_000);

        // At half-life blocks, approximately 50% remains
        let remaining = config.apply_decay(1_000_000, 1_000_000);
        assert!(remaining > 490_000 && remaining < 510_000); // ~500_000
    }
}
