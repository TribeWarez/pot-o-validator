pub(crate) mod auth;
mod config;
mod consensus;
mod device_registry;
mod extensions_bootstrap;
mod hex_api;
mod http_api;
mod internal_api;

use std::sync::Arc;

use config::ValidatorConfig;
use consensus::create_app_state;
use device_registry::{load_registry, DEFAULT_REGISTRY_PATH};
use hexchain_p2p::hex_consensus::HexConsensus;
use hexchain_p2p::types::{ConsensusParams, MmlParams};
use http_api::build_router;
use pot_o_extensions::{
    peer_network::RegistrationConfig, spawn_persist_ledger, LedgerEntry, DEFAULT_LEDGER_PATH,
};
use pot_o_mining::PotOConsensus;
use tokio::sync::RwLock;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pot_o_validator=debug,tower_http=debug".into()),
        )
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    let cfg = ValidatorConfig::load();
    tracing::info!(
        port = cfg.port,
        difficulty = cfg.difficulty,
        "Starting PoT-O Validator"
    );

    let consensus = PotOConsensus::new(cfg.difficulty, cfg.max_tensor_dim);
    let extensions = extensions_bootstrap::build_extension_registry(&cfg);

    let ledger_path =
        std::env::var("LEDGER_PATH").unwrap_or_else(|_| DEFAULT_LEDGER_PATH.to_string());
    spawn_persist_ledger(extensions.ledger.clone(), ledger_path.clone());

    let registry_path =
        std::env::var("DEVICE_REGISTRY_PATH").unwrap_or_else(|_| DEFAULT_REGISTRY_PATH.to_string());
    let device_registry = load_registry(&registry_path);

    let base_target_bytes: [u8; 32] = hex::decode(&cfg.base_target)
        .unwrap_or_else(|e| {
            tracing::warn!("Invalid BASE_TARGET hex, using default: {}", e);
            vec![0xFFu8; 32]
        })
        .try_into()
        .unwrap_or([0xFFu8; 32]);

    let consensus_params = ConsensusParams {
        maturity_depth: cfg.maturity_depth,
        symmetry_num: cfg.symmetry_num,
        symmetry_den: cfg.symmetry_den,
        base_target: base_target_bytes,
        mml: MmlParams::default(),
    };

    // ── Lattice persistence ───────────────────────────────────────────
    let lattice_path =
        std::env::var("LATTICE_PATH").unwrap_or_else(|_| "/blockstore/lattice.json".to_string());
    let hex_consensus = HexConsensus::new_with_path(consensus_params, &lattice_path);
    match hex_consensus.store.load_from_file() {
        Ok(n) => tracing::info!(entries = n, path = %lattice_path, "Hex lattice loaded from disk"),
        Err(e) => {
            tracing::info!(reason = %e, path = %lattice_path, "Hex lattice starting fresh (no saved state)")
        }
    }

    let network = extensions.network.clone();
    let mempool = extensions.mempool.clone();
    let ledger = extensions.ledger.clone();
    let tribechain_enabled = extensions.tribechain_enabled;
    let block_store = extensions.block_store.clone();

    let wallet_url = std::env::var("WALLET_URL").ok();
    if let Some(ref url) = wallet_url {
        tracing::info!(wallet_url = %url, "Wallet service integration enabled");
    } else {
        tracing::info!(
            "WALLET_URL not set — mining rewards will not be forwarded to wallet service"
        );
    }

    let state = create_app_state(
        cfg.clone(),
        consensus,
        extensions,
        registry_path,
        device_registry,
        hex_consensus,
        wallet_url,
    );

    if let Some(ref bs) = block_store {
        let bs = bs.clone();
        tokio::spawn(async move {
            let interval = tokio::time::Duration::from_secs(10);
            loop {
                tokio::time::sleep(interval).await;
                if bs.is_modified() {
                    if let Err(e) = bs.save_to_file() {
                        tracing::error!("Failed to persist block store: {}", e);
                    } else {
                        bs.clear_modified();
                    }
                }
            }
        });
        tracing::info!("BlockStore background persistence started");
    }

    // Persist hex lattice every 10 seconds (same cadence as block store)
    {
        let lattice = state.hex_consensus.store.clone();
        tokio::spawn(async move {
            let interval = tokio::time::Duration::from_secs(10);
            loop {
                tokio::time::sleep(interval).await;
                if let Err(e) = lattice.save_to_file() {
                    tracing::warn!(error = %e, "Failed to persist hex lattice");
                }
            }
        });
        tracing::info!(path = %lattice_path, "Hex lattice background persistence started");
    }

    if !cfg.bootstrap_urls.is_empty() && cfg.peer_network_mode == "vpn_mesh" {
        let reg_cfg = RegistrationConfig {
            node_id: cfg.node_id.clone(),
            address: cfg.listen_addr.clone(),
            port: cfg.port,
            version: VERSION.to_string(),
            bootstrap_urls: cfg.bootstrap_urls.clone(),
            timeout_secs: cfg.peer_timeout_secs,
        };
        let url_count = reg_cfg.bootstrap_urls.len();
        tokio::spawn(async move {
            let interval = tokio::time::Duration::from_secs(60);
            loop {
                let _ = network.register_with_bootstrap(&reg_cfg).await;
                tokio::time::sleep(interval).await;
            }
        });
        tracing::info!("Bootstrap registration task started ({} urls)", url_count);
    }

    let internal_state = internal_api::InternalApiState {
        node_id: cfg.node_id.clone(),
        peers: Arc::new(RwLock::new(Vec::new())),
        current_challenge: Arc::new(RwLock::new(None)),
        mempool,
        ledger,
        tribechain_enabled,
    };

    let app = build_router(Arc::clone(&state))
        .merge(hex_api::hex_routes(Arc::clone(&state)))
        .merge(internal_api::internal_router(internal_state));

    let addr = format!("{}:{}", cfg.listen_addr, cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Listening on {addr}");

    // ── Graceful shutdown: persist state on SIGTERM/SIGINT ────────────
    let shutdown_state = Arc::clone(&state);
    let shutdown_ledger = state.extensions.ledger.clone();
    let shutdown_ledger_path = ledger_path.clone();

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let sigterm = async {
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("failed to register SIGTERM handler")
                    .recv()
                    .await;
            };
            let sigint = async {
                tokio::signal::ctrl_c().await.expect("failed to register SIGINT handler");
            };
            tokio::select! {
                _ = sigterm => {},
                _ = sigint => {},
            }
            tracing::info!("Shutdown signal received — force-persisting state...");

            // Force-save ledger (write regardless of modified flag)
            {
                let l = shutdown_ledger.read().await;
                let entries: Vec<LedgerEntry> = l
                    .balances()
                    .iter()
                    .map(|((addr, token), bal)| LedgerEntry {
                        address: addr.clone(),
                        token: token.clone(),
                        balance: *bal,
                    })
                    .collect();
                if let Err(e) = std::fs::write(
                    &shutdown_ledger_path,
                    serde_json::to_string_pretty(&entries).unwrap(),
                ) {
                    tracing::error!("Failed to persist ledger on shutdown: {}", e);
                } else {
                    tracing::info!("Ledger saved to {}", shutdown_ledger_path);
                }
            }

            // Force-save block store
            if let Some(ref bs) = shutdown_state.extensions.block_store {
                if bs.is_modified() {
                    if let Err(e) = bs.save_to_file() {
                        tracing::error!("Failed to persist block store on shutdown: {}", e);
                    } else {
                        tracing::info!("BlockStore saved");
                    }
                }
            }

            // Force-save lattice
            if let Err(e) = shutdown_state.hex_consensus.store.save_to_file() {
                tracing::warn!("Failed to persist lattice on shutdown: {}", e);
            } else {
                tracing::info!("Hex lattice saved");
            }

            tracing::info!("State persistence complete — exiting.");
        })
        .await
        .unwrap();
}
