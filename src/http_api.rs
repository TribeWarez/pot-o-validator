//! HTTP API routes for the PoT-O validator: health, status, challenge, submit, devices, staking, swap, vault.

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use pot_o_extensions::DefiClient;
use pot_o_mining::{PotOProof, ProofPayload};
use serde::Deserialize;

use crate::consensus::AppState;
use crate::device_registry::{
    normalize_device_type, spawn_persist_registry, CurrentCalculation, RegisteredDevice,
    DEVICE_TYPE_KEYS,
};

/// Aggregated stats per device type: (count, proofs_valid, tasks_processed, last_activity).
type DeviceTypeStat = (u64, u64, u64, Option<chrono::DateTime<chrono::Utc>>);

/// Builds the Axum router with all validator routes (health, status, challenge, submit, devices, DeFi).
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(|| async { Redirect::permanent("/status") }))
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/challenge", post(get_challenge))
        .route("/submit", post(submit_proof))
        .route("/miners/{pubkey}", get(get_miner))
        .route("/pool", get(pool_info))
        .route("/devices/register", post(register_device))
        .route("/devices/progress", post(device_progress))
        .route("/devices", get(get_devices))
        .route("/network/peers", get(get_peers))
        // Staking (tribewarez-staking)
        .route("/staking/pool/:token_mint", get(get_staking_pool))
        .route(
            "/staking/stake/:pool_pubkey/:user_pubkey",
            get(get_stake_account),
        )
        // Swap (tribewarez-swap)
        .route("/swap/pool/:token_a/:token_b", get(get_swap_pool))
        .route("/swap/quote", get(get_swap_quote))
        // Vault (tribewarez-vault)
        .route("/vault/treasury/:token_mint", get(get_vault_treasury))
        .route(
            "/vault/vault/:treasury_pubkey/:user_pubkey",
            get(get_user_vault),
        )
        .route("/vault/escrow/:depositor/:beneficiary", get(get_escrow))
        .with_state(state)
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
    let engine_stats = state.consensus.engine_stats();
    let network = state.extensions.network.sync_state().await.ok();
    let peers = state
        .extensions
        .network
        .discover_peers()
        .await
        .ok()
        .unwrap_or_default();
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
    let slot_hash = body
        .slot_hash
        .unwrap_or_else(|| format!("{:0>64}", hex::encode(slot.to_le_bytes())));
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
            (
                StatusCode::OK,
                Json(serde_json::to_value(&challenge).unwrap()),
            )
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
    /// Optional device_type (cpu, native, gpu, esp32, esp8266, wasm). If set, registry is upserted so CPU/native/GPU stats update live even without prior /devices/register.
    device_type: Option<String>,
}

async fn submit_proof(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SubmitRequest>,
) -> impl IntoResponse {
    tracing::debug!(
        challenge_id = %body.proof.challenge_id,
        miner = %body.proof.miner_pubkey,
        device_id = ?body.device_id,
        device_type = ?body.device_type,
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
                let now = chrono::Utc::now();
                let device_type_normalized = body
                    .device_type
                    .as_deref()
                    .map(normalize_device_type)
                    .unwrap_or_else(|| "native".to_string());
                let registry_key: String = match &body.device_id {
                    Some(id) => id.clone(),
                    None => format!("{}:{}", body.proof.miner_pubkey, device_type_normalized),
                };
                const MAX_MINER_PUBKEYS_PER_DEVICE: usize = 100;
                {
                    let mut reg = state.device_registry.write().await;
                    let entry = reg.entry(registry_key).or_insert_with(|| RegisteredDevice {
                        device_type: device_type_normalized.clone(),
                        node_id: state.config.node_id.clone(),
                        last_activity: now,
                        proofs_valid: 0,
                        tasks_processed: 0,
                        miner_pubkeys: Vec::new(),
                        current_calculation: None,
                    });
                    entry.last_activity = now;
                    entry.proofs_valid += 1;
                    entry.tasks_processed += 1;
                    if body.device_id.is_some() {
                        entry.device_type = device_type_normalized;
                        let pk = body.proof.miner_pubkey.as_str();
                        if !entry.miner_pubkeys.iter().any(|p| p.as_str() == pk)
                            && entry.miner_pubkeys.len() < MAX_MINER_PUBKEYS_PER_DEVICE {
                                entry.miner_pubkeys.push(body.proof.miner_pubkey.clone());
                            }
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
                    Json(
                        serde_json::json!({ "accepted": false, "error": "Proof validation failed" }),
                    ),
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
    /// If provided and miner not yet on-chain, validator will auto-register the miner (relayer signs).
    miner_pubkey: Option<String>,
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
                    miner_pubkeys: Vec::new(),
                    current_calculation: None,
                },
            );
            true
        }
    };

    let miner_registered = if let Some(ref miner_pubkey) = body.miner_pubkey {
        match state.extensions.chain.query_miner(miner_pubkey).await {
            Ok(None) => match state.extensions.chain.register_miner(miner_pubkey).await {
                Ok(_) => {
                    tracing::info!(
                        device_id = %device_id,
                        miner_pubkey = %miner_pubkey,
                        "Auto-registered miner on-chain at device registration"
                    );
                    true
                }
                Err(e) => {
                    tracing::warn!(
                        device_id = %device_id,
                        miner_pubkey = %miner_pubkey,
                        error = %e,
                        "Auto-register miner at registration failed"
                    );
                    false
                }
            },
            Ok(Some(_)) => true, // already on-chain, can mine
            Err(e) => {
                tracing::warn!(
                    device_id = %device_id,
                    miner_pubkey = %miner_pubkey,
                    error = %e,
                    "Query miner failed at device registration"
                );
                false
            }
        }
    } else {
        false
    };

    tracing::info!(
        device_id = %device_id,
        device_type = %body.device_type,
        is_new = is_new,
        miner_registered = miner_registered,
        "POST /devices/register"
    );
    let reg = state.device_registry.read().await.clone();
    spawn_persist_registry(reg, state.registry_path.clone());
    Json(serde_json::json!({
        "registered": true,
        "device_type": body.device_type,
        "device_id": device_id,
        "miner_registered": miner_registered,
    }))
}

#[derive(Deserialize)]
struct DeviceProgressRequest {
    /// Optional device_id (e.g. MAC or UUID). If set, this device entry is updated.
    device_id: Option<String>,
    /// Optional miner_pubkey; used with device_type when device_id is not set to form registry key.
    miner_pubkey: Option<String>,
    /// Optional device_type (default "native"). Used with miner_pubkey when device_id is not set.
    device_type: Option<String>,
    /// Current challenge id the device/thread is working on.
    challenge_id: String,
    /// Hash of the current running calculation (e.g. state or work-in-progress).
    hash: String,
}

async fn device_progress(
    State(state): State<Arc<AppState>>,
    Json(body): Json<DeviceProgressRequest>,
) -> impl IntoResponse {
    let device_type_normalized = body
        .device_type
        .as_deref()
        .map(normalize_device_type)
        .unwrap_or_else(|| "native".to_string());
    let registry_key: Option<String> = match &body.device_id {
        Some(id) => Some(id.clone()),
        None => body
            .miner_pubkey
            .as_ref()
            .map(|pk| format!("{}:{}", pk, device_type_normalized)),
    };
    let Some(registry_key) = registry_key else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": "Either device_id or miner_pubkey must be set",
            })),
        );
    };
    let now = chrono::Utc::now();
    let current = CurrentCalculation {
        challenge_id: body.challenge_id,
        hash: body.hash,
        updated_at: now,
    };
    let updated = {
        let mut reg = state.device_registry.write().await;
        let entry = reg
            .entry(registry_key.clone())
            .or_insert_with(|| RegisteredDevice {
                device_type: device_type_normalized.clone(),
                node_id: state.config.node_id.clone(),
                last_activity: now,
                proofs_valid: 0,
                tasks_processed: 0,
                miner_pubkeys: Vec::new(),
                current_calculation: None,
            });
        entry.last_activity = now;
        entry.current_calculation = Some(current);
        true
    };
    let reg = state.device_registry.read().await.clone();
    spawn_persist_registry(reg, state.registry_path.clone());
    tracing::debug!(registry_key = %registry_key, "POST /devices/progress");
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "ok": true,
            "updated": updated,
        })),
    )
}

async fn get_devices(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    tracing::debug!("GET /devices");
    let reg = state.device_registry.read().await.clone();
    let mut by_type: HashMap<String, DeviceTypeStat> =
        HashMap::new();
    for key in DEVICE_TYPE_KEYS {
        by_type.insert((*key).to_string(), (0, 0, 0, None));
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
    // Per-device detail for analytics (includes miner_pubkeys and current_calculation when keyed by device_id).
    let devices_detail: serde_json::Map<String, serde_json::Value> = reg
        .iter()
        .map(|(id, d)| {
            let current_calculation = d.current_calculation.as_ref().map(|c| {
                serde_json::json!({
                    "challenge_id": c.challenge_id,
                    "hash": c.hash,
                    "updated_at": c.updated_at.to_rfc3339(),
                })
            });
            (
                id.clone(),
                serde_json::json!({
                    "device_type": d.device_type,
                    "proofs_valid": d.proofs_valid,
                    "tasks_processed": d.tasks_processed,
                    "last_activity": d.last_activity.to_rfc3339(),
                    "miner_pubkeys": d.miner_pubkeys,
                    "current_calculation": current_calculation,
                }),
            )
        })
        .collect();

    tracing::debug!(device_count = reg.len(), "GET /devices response");
    Json(serde_json::json!({
        "miners_by_device": miners_map,
        "devices_detail": devices_detail,
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

// ---------------------------------------------------------------------------
// Staking / Swap / Vault (DeFi) handlers — run RPC in spawn_blocking
// ---------------------------------------------------------------------------

async fn get_staking_pool(
    State(state): State<Arc<AppState>>,
    Path(token_mint): Path<String>,
) -> impl IntoResponse {
    let rpc_url = state.config.solana_rpc_url.clone();
    match tokio::task::spawn_blocking(move || {
        let client = DefiClient::new(rpc_url);
        client.get_staking_pool(&token_mint)
    })
    .await
    {
        Ok(Ok(Some(pool))) => (StatusCode::OK, Json(serde_json::to_value(&pool).unwrap())),
        Ok(Ok(None)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Staking pool not found" })),
        ),
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "GET /staking/pool failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

async fn get_stake_account(
    State(state): State<Arc<AppState>>,
    Path((pool_pubkey, user_pubkey)): Path<(String, String)>,
) -> impl IntoResponse {
    let rpc_url = state.config.solana_rpc_url.clone();
    match tokio::task::spawn_blocking(move || {
        let client = DefiClient::new(rpc_url);
        client.get_stake_account(&pool_pubkey, &user_pubkey)
    })
    .await
    {
        Ok(Ok(Some(account))) => (
            StatusCode::OK,
            Json(serde_json::to_value(&account).unwrap()),
        ),
        Ok(Ok(None)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Stake account not found" })),
        ),
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "GET /staking/stake failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

async fn get_swap_pool(
    State(state): State<Arc<AppState>>,
    Path((token_a, token_b)): Path<(String, String)>,
) -> impl IntoResponse {
    let rpc_url = state.config.solana_rpc_url.clone();
    match tokio::task::spawn_blocking(move || {
        let client = DefiClient::new(rpc_url);
        client.get_swap_pool(&token_a, &token_b)
    })
    .await
    {
        Ok(Ok(Some(pool))) => (StatusCode::OK, Json(serde_json::to_value(&pool).unwrap())),
        Ok(Ok(None)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Liquidity pool not found" })),
        ),
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "GET /swap/pool failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct SwapQuoteQuery {
    token_a: String,
    token_b: String,
    amount_in: u64,
    is_a_to_b: Option<bool>,
}

async fn get_swap_quote(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SwapQuoteQuery>,
) -> impl IntoResponse {
    let rpc_url = state.config.solana_rpc_url.clone();
    let token_a = q.token_a.clone();
    let token_b = q.token_b.clone();
    let amount_in = q.amount_in;
    let is_a_to_b = q.is_a_to_b.unwrap_or(true);
    match tokio::task::spawn_blocking(move || {
        let client = DefiClient::new(rpc_url);
        client.get_swap_quote(&token_a, &token_b, amount_in, is_a_to_b)
    })
    .await
    {
        Ok(Ok(Some(quote))) => (StatusCode::OK, Json(serde_json::to_value(&quote).unwrap())),
        Ok(Ok(None)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Pool not found or no liquidity" })),
        ),
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "GET /swap/quote failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

async fn get_vault_treasury(
    State(state): State<Arc<AppState>>,
    Path(token_mint): Path<String>,
) -> impl IntoResponse {
    let rpc_url = state.config.solana_rpc_url.clone();
    match tokio::task::spawn_blocking(move || {
        let client = DefiClient::new(rpc_url);
        client.get_treasury(&token_mint)
    })
    .await
    {
        Ok(Ok(Some(treasury))) => (
            StatusCode::OK,
            Json(serde_json::to_value(&treasury).unwrap()),
        ),
        Ok(Ok(None)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Treasury not found" })),
        ),
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "GET /vault/treasury failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

async fn get_user_vault(
    State(state): State<Arc<AppState>>,
    Path((treasury_pubkey, user_pubkey)): Path<(String, String)>,
) -> impl IntoResponse {
    let rpc_url = state.config.solana_rpc_url.clone();
    match tokio::task::spawn_blocking(move || {
        let client = DefiClient::new(rpc_url);
        client.get_user_vault(&treasury_pubkey, &user_pubkey)
    })
    .await
    {
        Ok(Ok(Some(vault))) => (StatusCode::OK, Json(serde_json::to_value(&vault).unwrap())),
        Ok(Ok(None)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "User vault not found" })),
        ),
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "GET /vault/vault failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

async fn get_escrow(
    State(state): State<Arc<AppState>>,
    Path((depositor, beneficiary)): Path<(String, String)>,
) -> impl IntoResponse {
    let rpc_url = state.config.solana_rpc_url.clone();
    match tokio::task::spawn_blocking(move || {
        let client = DefiClient::new(rpc_url);
        client.get_escrow(&depositor, &beneficiary)
    })
    .await
    {
        Ok(Ok(Some(escrow))) => (StatusCode::OK, Json(serde_json::to_value(&escrow).unwrap())),
        Ok(Ok(None)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Escrow not found" })),
        ),
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "GET /vault/escrow failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}
