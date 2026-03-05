mod config;
mod consensus;
mod device_registry;
mod extensions_bootstrap;
mod http_api;

use std::sync::Arc;

use config::ValidatorConfig;
use consensus::create_app_state;
use device_registry::{load_registry, DEFAULT_REGISTRY_PATH};
use http_api::build_router;
use pot_o_mining::PotOConsensus;

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

    let registry_path =
        std::env::var("DEVICE_REGISTRY_PATH").unwrap_or_else(|_| DEFAULT_REGISTRY_PATH.to_string());
    let device_registry = load_registry(&registry_path);

    let state = create_app_state(
        cfg.clone(),
        consensus,
        extensions,
        registry_path,
        device_registry,
    );

    let app = build_router(Arc::clone(&state));

    let addr = format!("{}:{}", cfg.listen_addr, cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Listening on {addr}");
    axum::serve(listener, app).await.unwrap();
}
