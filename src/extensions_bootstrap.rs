//! Builds the extension registry from validator config (chain, pool, network).

use pot_o_extensions::ExtensionRegistry;

use crate::config::ValidatorConfig;

/// Builds an [`ExtensionRegistry`] from config with peer network mode and pool strategy support.
pub fn build_extension_registry(cfg: &ValidatorConfig) -> ExtensionRegistry {
    ExtensionRegistry::from_config(
        &cfg.solana_rpc_url,
        &cfg.pot_program_id,
        &cfg.relayer_keypair_path,
        cfg.auto_register_miners,
        &cfg.peer_network_mode,
        &cfg.pool_strategy,
    )
}
