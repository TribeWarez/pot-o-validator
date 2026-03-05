use pot_o_extensions::ExtensionRegistry;

use crate::config::ValidatorConfig;

pub fn build_extension_registry(cfg: &ValidatorConfig) -> ExtensionRegistry {
    ExtensionRegistry::local_defaults(
        &cfg.solana_rpc_url,
        &cfg.pot_program_id,
        &cfg.relayer_keypair_path,
        cfg.auto_register_miners,
    )
}
