//! PoT-O Validator binary: HTTP API server for challenge issuance, proof submission, and status.
//!
//! Loads config, builds consensus and extension registry, binds to the configured address/port,
//! and serves the routes defined in `http_api`.

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
use pot_o_extensions::{load_or_create_tribe_mint, spawn_persist_ledger, DEFAULT_LEDGER_PATH};
use pot_o_mining::PotOConsensus;
use tokio::sync::RwLock;

/// Crate version (from Cargo.toml).
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    // Full info/debug by default; use RUST_LOG=pot_o_validator=trace for trace, or RUST_LOG=warn to reduce
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
    spawn_persist_ledger(extensions.ledger.clone(), ledger_path);

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

    let hex_consensus = HexConsensus::new(consensus_params);

    let tribe_mint_address = load_or_create_tribe_mint(&cfg.tribe_mint_keypair_path);
    tracing::info!(address = %tribe_mint_address, "TRIBE mint");

    let mempool = extensions.mempool.clone();
    let ledger = extensions.ledger.clone();
    let tribechain_enabled = extensions.tribechain_enabled;
    let block_store = extensions.block_store.clone();

    let state = create_app_state(
        cfg.clone(),
        consensus,
        extensions,
        registry_path,
        device_registry,
        hex_consensus,
        tribe_mint_address,
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
    axum::serve(listener, app).await.unwrap();
}
