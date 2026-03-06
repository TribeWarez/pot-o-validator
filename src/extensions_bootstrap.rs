//! Builds the extension registry from validator config (chain, pool, network).

use pot_o_extensions::ExtensionRegistry;

use crate::config::ValidatorConfig;

/// Builds an [`ExtensionRegistry`] with local defaults using config (Solana RPC, program ID, relayer keypair, auto-register).
pub fn build_extension_registry(cfg: &ValidatorConfig) -> ExtensionRegistry {
    ExtensionRegistry::local_defaults(
        &cfg.solana_rpc_url,
        &cfg.pot_program_id,
        &cfg.relayer_keypair_path,
        cfg.auto_register_miners,
    )
}
