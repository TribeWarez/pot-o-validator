//! Builds the extension registry from validator config (chain, pool, network, ledger, tribechain).

use hexchain_p2p::block_store::BlockStore;
use pot_o_extensions::{load_ledger, ExtensionRegistry, DEFAULT_LEDGER_PATH};
use pot_o_extensions::{Genesis, Mempool};
use std::path::Path;
use std::sync::Arc;

use crate::config::ValidatorConfig;

/// Builds an [`ExtensionRegistry`] from config with peer network mode and pool strategy support.
/// When `tribechain_enabled` is true, also wires the mempool, block store, and genesis.
pub fn build_extension_registry(cfg: &ValidatorConfig) -> ExtensionRegistry {
    let ledger_path =
        std::env::var("LEDGER_PATH").unwrap_or_else(|_| DEFAULT_LEDGER_PATH.to_string());
    let mut ledger = load_ledger(&ledger_path, &cfg.protocol_fee_address);

    let genesis: Option<Genesis> =
        if cfg.tribechain_enabled && !cfg.tribechain_genesis_path.is_empty() {
            let blockstore_path = &cfg.tribechain_blockstore_path;
            let is_fresh_chain = !Path::new(blockstore_path).exists();

            match Genesis::load(&cfg.tribechain_genesis_path) {
                Ok(g) => {
                    if let Err(e) = g.validate() {
                        tracing::warn!("Invalid genesis file (tribechain disabled): {}", e);
                        None
                    } else if is_fresh_chain {
                        if let Err(e) = g.apply_to_ledger(&mut ledger) {
                            tracing::warn!("Failed to apply genesis to ledger: {}", e);
                            None
                        } else {
                            tracing::info!(
                                "Initialized ledger from genesis: {} entries, chain_id={}",
                                g.entries.len(),
                                g.chain_id
                            );
                            Some(g)
                        }
                    } else {
                        tracing::info!("Chain already initialized, skipping genesis apply");
                        Some(g)
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to load genesis (tribechain disabled): {}", e);
                    None
                }
            }
        } else {
            None
        };

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
