//! Builds the extension registry from validator config (chain, pool, network, ledger, tribechain).

use hexchain_p2p::block_store::BlockStore;
use pot_o_extensions::{genesis::Genesis, Mempool};
use pot_o_extensions::{load_ledger, ExtensionRegistry, DEFAULT_LEDGER_PATH};
use std::sync::Arc;

use crate::config::ValidatorConfig;

/// Builds an [`ExtensionRegistry`] from config with peer network mode and pool strategy support.
/// When `tribechain_enabled` is true, also wires the mempool, block store, and genesis.
pub fn build_extension_registry(cfg: &ValidatorConfig) -> ExtensionRegistry {
    let ledger_path =
        std::env::var("LEDGER_PATH").unwrap_or_else(|_| DEFAULT_LEDGER_PATH.to_string());
    let ledger = load_ledger(&ledger_path, &cfg.protocol_fee_address);

    let mut registry = ExtensionRegistry::from_config(
        &cfg.solana_rpc_url,
        &cfg.pot_program_id,
        &cfg.relayer_keypair_path,
        cfg.auto_register_miners,
        &cfg.peer_network_mode,
        &cfg.pool_strategy,
        &cfg.device_protocol,
        &cfg.protocol_fee_address,
        cfg.marketplace_fee_bps,
        Some(ledger),
        &cfg.bootstrap_urls,
        cfg.enable_mdns,
        &cfg.mdns_service_name,
        cfg.peer_timeout_secs,
        cfg.challenge_relay_enabled,
    );

    if cfg.tribechain_enabled {
        let genesis = if !cfg.tribechain_genesis_path.is_empty() {
            Some(Genesis::load(&cfg.tribechain_genesis_path))
        } else {
            None
        };
        registry.mempool = Some(Arc::new(Mempool::new(
            cfg.tribechain_max_pool_size,
            cfg.tribechain_min_fee,
        )));
        registry.block_store = Some(Arc::new(BlockStore::new(&cfg.tribechain_blockstore_path)));
        registry.genesis = genesis;
        registry.tribechain_enabled = true;
    }

    registry
}
