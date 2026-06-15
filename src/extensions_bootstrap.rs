//! Builds the extension registry from validator config (chain, pool, network, ledger).

use pot_o_extensions::{load_ledger, ExtensionRegistry, DEFAULT_LEDGER_PATH};

use crate::config::ValidatorConfig;

/// Builds an [`ExtensionRegistry`] from config with peer network mode and pool strategy support.
pub fn build_extension_registry(cfg: &ValidatorConfig) -> ExtensionRegistry {
    let ledger_path =
        std::env::var("LEDGER_PATH").unwrap_or_else(|_| DEFAULT_LEDGER_PATH.to_string());
    let ledger = load_ledger(&ledger_path, &cfg.protocol_fee_address);

    ExtensionRegistry::from_config(
        &cfg.solana_rpc_url,
        &cfg.pot_program_id,
        &cfg.relayer_keypair_path,
        cfg.auto_register_miners,
        &cfg.peer_network_mode,
        &cfg.pool_strategy,
        &cfg.protocol_fee_address,
        cfg.marketplace_fee_bps,
        Some(ledger),
    )
}
