mod config;

use axum::{
    extract::Path,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use pot_o_extensions::{ExtensionRegistry, PoolStrategy as _};
use pot_o_mining::{PotOConsensus, PotOProof, ProofPayload};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use config::ValidatorConfig;

/// Per-device record for the registry (device_id -> device state).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegisteredDevice {
    device_type: String,
    node_id: String,
    last_activity: chrono::DateTime<chrono::Utc>,
    proofs_valid: u64,
    tasks_processed: u64,
}

fn normalize_device_type(s: &str) -> String {
    let lower = s.to_lowercase();
    if lower == "esp32" || lower == "esp32s" {
        return "esp32".to_string();
    }
    if lower == "esp8266" {
        return "esp8266".to_string();
    }
    if lower == "gpu" || lower == "cuda" || lower == "opencl" {
        return "gpu".to_string();
    }
    if lower == "cpu" {
        return "cpu".to_string();
    }
    if lower == "native" {
        return "native".to_string();
    }
    if lower == "wasm" {
        return "wasm".to_string();
    }
    "native".to_string()
}

const DEVICE_TYPE_KEYS: &[&str] = &["esp32", "esp8266", "gpu", "cpu", "native", "wasm"];
const DEFAULT_REGISTRY_PATH: &str = "device_registry.json";

struct AppState {
    config: ValidatorConfig,
    consensus: PotOConsensus,
    extensions: ExtensionRegistry,
    current_challenge: RwLock<Option<pot_o_mining::Challenge>>,
    stats: RwLock<ValidatorStats>,
    /// device_id (e.g. MAC) -> RegisteredDevice. Persisted so ESP mappings survive restarts.
    device_registry: RwLock<HashMap<String, RegisteredDevice>>,
    registry_path: String,
}

fn load_registry(path: &str) -> HashMap<String, RegisteredDevice> {
    match std::fs::read_to_string(path) {
        Ok(data) => {
            let reg: HashMap<String, RegisteredDevice> = serde_json::from_str(&data).unwrap_or_default();
            tracing::debug!(path, count = reg.len(), "Loaded device registry from disk");
            reg
        }
        Err(_) => {
            tracing::debug!(path, "No existing device registry file; starting empty");
            HashMap::new()
        }
    }
}

fn spawn_persist_registry(reg: HashMap<String, RegisteredDevice>, path: String) {
    let count = reg.len();
    tokio::spawn(async move {
        if let Ok(json) = serde_json::to_string_pretty(&reg) {
            if let Err(e) = tokio::fs::write(&path, json).await {
                tracing::warn!(path, error = %e, "Failed to persist device registry");
            } else {
                tracing::debug!(path, count, "Persisted device registry");
            }
        }
    });
}

#[derive(Debug, Clone, Default, Serialize)]
struct ValidatorStats {
    total_challenges_issued: u64,
    total_proofs_received: u64,
    total_proofs_valid: u64,
    active_miners: u64,
    uptime_secs: u64,
    /// Paths validated in the current challenge round (reset on new challenge).
    paths_in_block: u64,
    /// Tensor computations completed in the current challenge round (reset on new challenge).
    calcs_in_block: u64,
}

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
    tracing::info!(port = cfg.port, difficulty = cfg.difficulty, "Starting PoT-O Validator");

    let consensus = PotOConsensus::new(cfg.difficulty, cfg.max_tensor_dim);
    let extensions = ExtensionRegistry::local_defaults(
        &cfg.solana_rpc_url,
        &cfg.pot_program_id,
        &cfg.relayer_keypair_path,
    );

    let registry_path = std::env::var("DEVICE_REGISTRY_PATH")
        .unwrap_or_else(|_| DEFAULT_REGISTRY_PATH.to_string());
    let device_registry = load_registry(&registry_path);

    let state = Arc::new(AppState {
        config: cfg.clone(),
        consensus,
        extensions,
        current_challenge: RwLock::new(None),
        stats: RwLock::new(ValidatorStats::default()),
        device_registry: RwLock::new(device_registry),
        registry_path,
    });

    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/status") }))
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/challenge", post(get_challenge))
        .route("/submit", post(submit_proof))
        .route("/miners/{pubkey}", get(get_miner))
        .route("/pool", get(pool_info))
        .route("/devices/register", post(register_device))
        .route("/devices", get(get_devices))
        .route("/network/peers", get(get_peers))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("{}:{}", cfg.listen_addr, cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Listening on {addr}");
    axum::serve(listener, app).await.unwrap();
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "pot-o-validator",
        "version": pot_o_validator::VERSION,
    }))
}

async fn status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    tracing::debug!("GET /status");
    let stats = state.stats.read().await.clone();
    let engine_stats = state.consensus.engine.get_stats();
    let network = state.extensions.network.sync_state().await.ok();
    let peers = state.extensions.network.discover_peers().await.ok().unwrap_or_default();
    let current_challenge = state.current_challenge.read().await.as_ref().map(|c| {
        let (expected_paths, expected_calcs) = state.consensus.expected_paths_and_calcs(c);
        serde_json::json!({
            "id": c.id,
            "slot": c.slot,
            "operation_type": c.operation_type,
            "difficulty": c.difficulty,
            "mml_threshold": c.mml_threshold,
            "path_distance_max": c.path_distance_max,
            "expires_at": c.expires_at.to_rfc3339(),
            "expected_paths": expected_paths,
            "expected_calcs": expected_calcs,
        })
    });
    let resp = Json(serde_json::json!({
        "node_id": state.config.node_id,
        "difficulty": state.config.difficulty,
        "max_tensor_dim": state.config.max_tensor_dim,
        "peer_network_mode": state.config.peer_network_mode,
        "pool_strategy": state.config.pool_strategy,
        "stats": stats,
        "engine": {
            "tasks_processed": engine_stats.total_tasks_processed,
            "successful": engine_stats.successful_tasks,
            "failed": engine_stats.failed_tasks,
        },
        "network": network,
        "current_challenge": current_challenge,
        "connected_peers": peers,
    }));
    tracing::info!(
        challenges_issued = stats.total_challenges_issued,
        proofs_valid = stats.total_proofs_valid,
        paths_in_block = stats.paths_in_block,
        calcs_in_block = stats.calcs_in_block,
        peers = peers.len(),
        has_challenge = current_challenge.is_some(),
        "GET /status response"
    );
    resp
}

#[derive(Deserialize)]
struct ChallengeRequest {
    slot: Option<u64>,
    slot_hash: Option<String>,
    device_type: Option<String>,
}

async fn get_challenge(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ChallengeRequest>,
) -> impl IntoResponse {
    let slot = body.slot.unwrap_or(100);
    let slot_hash = body.slot_hash.unwrap_or_else(|| {
        format!("{:0>64}", hex::encode(slot.to_le_bytes()))
    });
    tracing::debug!(slot, device_type = ?body.device_type, "POST /challenge request");

    match state.consensus.generate_challenge(slot, &slot_hash) {
        Ok(challenge) => {
            {
                let mut s = state.stats.write().await;
                s.total_challenges_issued += 1;
                s.paths_in_block = 0;
                s.calcs_in_block = 0;
            }
            let mut current = state.current_challenge.write().await;
            *current = Some(challenge.clone());
            tracing::info!(
                challenge_id = %challenge.id,
                slot = challenge.slot,
                operation_type = %challenge.operation_type,
                difficulty = challenge.difficulty,
                "POST /challenge issued"
            );
            (StatusCode::OK, Json(serde_json::to_value(&challenge).unwrap()))
        }
        Err(e) => {
            tracing::warn!(error = %e, "POST /challenge failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        }
    }
}

#[derive(Deserialize)]
struct SubmitRequest {
    proof: PotOProof,
    signature: Option<Vec<u8>>,
    /// Optional device_id (e.g. MAC) for real-time ESP mapping; updates registry on success.
    device_id: Option<String>,
}

async fn submit_proof(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SubmitRequest>,
) -> impl IntoResponse {
    tracing::debug!(
        challenge_id = %body.proof.challenge_id,
        miner = %body.proof.miner_pubkey,
        device_id = ?body.device_id,
        "POST /submit received"
    );
    {
        let mut s = state.stats.write().await;
        s.total_proofs_received += 1;
    }

    let challenge = state.current_challenge.read().await;
    if let Some(ref chal) = *challenge {
        match state.consensus.verify_proof(&body.proof, chal) {
            Ok(true) => {
                {
                    let mut s = state.stats.write().await;
                    s.total_proofs_valid += 1;
                    s.paths_in_block += 1;
                    s.calcs_in_block += 1;
                }
                if let Some(ref device_id) = body.device_id {
                    let mut reg = state.device_registry.write().await;
                    if let Some(dev) = reg.get_mut(device_id) {
                        dev.last_activity = chrono::Utc::now();
                        dev.proofs_valid += 1;
                        dev.tasks_processed += 1;
                    }
                }
                {
                    let reg = state.device_registry.read().await.clone();
                    spawn_persist_registry(reg, state.registry_path.clone());
                }

                let payload = ProofPayload {
                    proof: body.proof.clone(),
                    signature: body.signature.unwrap_or_default(),
                };

                match state.extensions.chain.submit_proof(&payload).await {
                    Ok(tx) => {
                        tracing::info!(
                            challenge_id = %body.proof.challenge_id,
                            tx_signature = %tx.0,
                            device_id = ?body.device_id,
                            "POST /submit accepted (on-chain)"
                        );
                        (
                            StatusCode::OK,
                            Json(serde_json::json!({
                                "accepted": true,
                                "tx_signature": tx.0,
                            })),
                        )
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "POST /submit chain submit failed");
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({ "accepted": false, "error": e.to_string() })),
                        )
                    }
                }
            }
            Ok(false) => {
                tracing::info!(challenge_id = %body.proof.challenge_id, "POST /submit rejected (validation failed)");
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "accepted": false, "error": "Proof validation failed" })),
                )
            }
            Err(e) => {
                tracing::warn!(error = %e, "POST /submit verify error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "accepted": false, "error": e.to_string() })),
                )
            }
        }
    } else {
        tracing::debug!("POST /submit rejected (no active challenge)");
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "accepted": false, "error": "No active challenge" })),
        )
    }
}

async fn get_miner(
    State(state): State<Arc<AppState>>,
    Path(pubkey): Path<String>,
) -> impl IntoResponse {
    tracing::debug!(pubkey = %pubkey, "GET /miners/:pubkey");
    match state.extensions.chain.query_miner(&pubkey).await {
        Ok(Some(acct)) => {
            tracing::debug!(pubkey = %pubkey, "GET /miners/:pubkey found");
            (StatusCode::OK, Json(serde_json::to_value(&acct).unwrap()))
        }
        Ok(None) => {
            tracing::debug!(pubkey = %pubkey, "GET /miners/:pubkey not found");
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Miner not found" })),
            )
        }
        Err(e) => {
            tracing::warn!(pubkey = %pubkey, error = %e, "GET /miners/:pubkey error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        }
    }
}

async fn pool_info(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    tracing::debug!("GET /pool");
    let info = state.extensions.pool.pool_info(0, 0);
    Json(serde_json::to_value(&info).unwrap())
}

#[derive(Deserialize)]
struct DeviceRegisterRequest {
    device_type: String,
    device_id: Option<String>,
}

async fn register_device(
    State(state): State<Arc<AppState>>,
    Json(body): Json<DeviceRegisterRequest>,
) -> impl IntoResponse {
    let device_id = body
        .device_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let device_type_normalized = normalize_device_type(&body.device_type);
    let now = chrono::Utc::now();
    let is_new = {
        let mut reg = state.device_registry.write().await;
        if let Some(prev) = reg.get_mut(&device_id) {
            prev.last_activity = now;
            prev.device_type = device_type_normalized.clone();
            false
        } else {
            reg.insert(
                device_id.clone(),
                RegisteredDevice {
                    device_type: device_type_normalized,
                    node_id: state.config.node_id.clone(),
                    last_activity: now,
                    proofs_valid: 0,
                    tasks_processed: 0,
                },
            );
            true
        }
    };
    tracing::info!(
        device_id = %device_id,
        device_type = %body.device_type,
        is_new = is_new,
        "POST /devices/register"
    );
    let reg = state.device_registry.read().await.clone();
    spawn_persist_registry(reg, state.registry_path.clone());
    Json(serde_json::json!({
        "registered": true,
        "device_type": body.device_type,
        "device_id": device_id,
    }))
}

async fn get_devices(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    tracing::debug!("GET /devices");
    let reg = state.device_registry.read().await.clone();
    let mut by_type: HashMap<String, (u64, u64, u64, Option<chrono::DateTime<chrono::Utc>>)> =
        HashMap::new();
    for key in DEVICE_TYPE_KEYS {
        by_type.insert(
            (*key).to_string(),
            (0, 0, 0, None),
        );
    }
    for d in reg.values() {
        let key = &d.device_type;
        if !DEVICE_TYPE_KEYS.contains(&key.as_str()) {
            continue;
        }
        let entry = by_type.get_mut(key).unwrap();
        entry.0 += 1;
        entry.1 += d.proofs_valid;
        entry.2 += d.tasks_processed;
        if entry.3.is_none() || d.last_activity > entry.3.unwrap() {
            entry.3 = Some(d.last_activity);
        }
    }
    let mut miners_map = serde_json::Map::new();
    for (k, (count, proofs_valid, tasks_processed, last_activity)) in by_type {
        let last_activity_val = last_activity.map(|t| serde_json::Value::String(t.to_rfc3339()));
        let proofs_val = if count > 0 {
            serde_json::Value::Number(serde_json::Number::from(proofs_valid))
        } else {
            serde_json::Value::Null
        };
        let tasks_val = if count > 0 {
            serde_json::Value::Number(serde_json::Number::from(tasks_processed))
        } else {
            serde_json::Value::Null
        };
        miners_map.insert(
            k,
            serde_json::json!({
                "count": count,
                "proofs_valid": proofs_val,
                "tasks_processed": tasks_val,
                "last_activity": last_activity_val,
            }),
        );
    }
    tracing::debug!(device_count = reg.len(), "GET /devices response");
    Json(serde_json::json!({
        "miners_by_device": miners_map,
    }))
}

async fn get_peers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    tracing::debug!("GET /network/peers");
    match state.extensions.network.discover_peers().await {
        Ok(peers) => {
            tracing::debug!(peer_count = peers.len(), "GET /network/peers");
            Json(serde_json::json!({
                "node_id": state.extensions.network.node_id(),
                "peers": peers,
            }))
        }
        Err(e) => {
            tracing::warn!(error = %e, "GET /network/peers failed");
            Json(serde_json::json!({
                "error": e.to_string(),
            }))
        }
    }
}
