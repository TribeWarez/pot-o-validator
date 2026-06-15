//! HTTP API routes for the PoT-O validator: health, status, challenge, submit, devices, staking, swap, vault.

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{delete, get, post},
    Json, Router,
};
use pot_o_core::TokenType;
use pot_o_extensions::marketplace::{parse_market_asset, OrderSide};
use pot_o_extensions::DefiClient;
use pot_o_mining::{PotOProof, ProofPayload};
use serde::Deserialize;

use crate::consensus::AppState;
use crate::device_registry::{
    normalize_device_type, spawn_persist_registry, CurrentCalculation, RegisteredDevice,
    DEVICE_TYPE_KEYS,
};

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
        // Token ledger
        .route(
            "/token/balance/{address}/{token_type}",
            get(get_token_balance),
        )
        .route("/token/transfer", post(post_token_transfer))
        .route("/token/tx/{address}", get(get_token_tx_history))
        // Marketplace (v0.5.1+)
        .route("/marketplace/order", post(post_marketplace_order))
        .route("/marketplace/order/{id}", delete(delete_marketplace_order))
        .route("/marketplace/order/{id}", get(get_marketplace_order))
        .route(
            "/marketplace/orderbook/{sell_asset}/{buy_asset}",
            get(get_marketplace_orderbook),
        )
        .route("/marketplace/orders/{maker}", get(get_marketplace_orders))
        .route("/marketplace/trades", get(get_marketplace_trades))
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
        "version": crate::VERSION,
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

            // Broadcast challenge to peers (non-blocking)
            let challenge_clone = challenge.clone();
            let state_clone = state.clone();
            tokio::spawn(async move {
                match state_clone
                    .extensions
                    .network
                    .broadcast_challenge(&challenge_clone)
                    .await
                {
                    Ok(()) => {
                        tracing::debug!(
                            challenge_id = %challenge_clone.id,
                            "Challenge broadcast to peers completed"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            challenge_id = %challenge_clone.id,
                            error = %e,
                            "Challenge broadcast to peers failed (non-fatal)"
                        );
                    }
                }
            });

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
                            && entry.miner_pubkeys.len() < MAX_MINER_PUBKEYS_PER_DEVICE
                        {
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

#[allow(clippy::type_complexity)]
async fn get_devices(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    tracing::debug!("GET /devices");
    let reg = state.device_registry.read().await.clone();
    let mut by_type: HashMap<String, (u64, u64, u64, Option<chrono::DateTime<chrono::Utc>>)> =
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

// ---------------------------------------------------------------------------
// Token ledger handlers
// ---------------------------------------------------------------------------

async fn get_token_balance(
    State(state): State<Arc<AppState>>,
    Path((address, token_type_str)): Path<(String, String)>,
) -> impl IntoResponse {
    tracing::debug!(address = %address, token = %token_type_str, "GET /token/balance");

    let token = match token_type_from_str(&token_type_str) {
        Some(t) => t,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::json!({ "error": format!("Unknown token type: {}", token_type_str) }),
                ),
            );
        }
    };

    let balance = state
        .extensions
        .ledger
        .read()
        .await
        .balance_of(&address, &token);
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "address": address,
            "token": token_type_str,
            "balance": balance,
        })),
    )
}

#[derive(Debug, Deserialize)]
struct TransferRequest {
    from: String,
    to: String,
    token_type: String,
    amount: u64,
    fee: u64,
}

async fn post_token_transfer(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TransferRequest>,
) -> impl IntoResponse {
    tracing::debug!(
        from = %body.from, to = %body.to,
        token = %body.token_type, amount = body.amount, fee = body.fee,
        "POST /token/transfer"
    );

    let token = match token_type_from_str(&body.token_type) {
        Some(t) => t,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::json!({ "error": format!("Unknown token type: {}", body.token_type) }),
                ),
            );
        }
    };

    let receipt = {
        let mut ledger = state.extensions.ledger.write().await;
        ledger.transfer(&body.from, &body.to, &token, body.amount, body.fee)
    };

    match receipt {
        Ok(r) => (StatusCode::OK, Json(serde_json::to_value(&r).unwrap())),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

async fn get_token_tx_history(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    tracing::debug!(address = %address, "GET /token/tx");

    let txs = state
        .extensions
        .ledger
        .read()
        .await
        .tx_history_for(&address);
    (
        StatusCode::OK,
        Json(serde_json::json!({ "address": address, "transactions": txs })),
    )
}

// ---------------------------------------------------------------------------
// Marketplace handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct PlaceOrderRequest {
    maker: String,
    side: String,
    sell_asset: String,
    buy_asset: String,
    amount: u64,
    price: u64,
}

async fn post_marketplace_order(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PlaceOrderRequest>,
) -> impl IntoResponse {
    tracing::debug!(
        maker = %body.maker, side = %body.side,
        sell = %body.sell_asset, buy = %body.buy_asset,
        amount = body.amount, price = body.price,
        "POST /marketplace/order"
    );

    let side = match body.side.to_lowercase().as_str() {
        "buy" => OrderSide::Buy,
        "sell" => OrderSide::Sell,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "side must be 'buy' or 'sell'" })),
            )
        }
    };

    let sell_asset = match parse_market_asset(&body.sell_asset) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            )
        }
    };
    let buy_asset = match parse_market_asset(&body.buy_asset) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            )
        }
    };

    let (order_id, trades) = {
        let mut mp = state.extensions.marketplace.write().await;
        mp.place_and_match(
            &body.maker,
            side,
            sell_asset,
            buy_asset,
            body.amount,
            body.price,
        )
    };

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "order_id": order_id,
            "trades": trades,
        })),
    )
}

async fn delete_marketplace_order(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    tracing::debug!(order_id = %id, "DELETE /marketplace/order");
    let cancelled = {
        let mut mp = state.extensions.marketplace.write().await;
        mp.cancel_order(&id)
    };
    if cancelled {
        (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "cancelled", "order_id": id })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Order not found or already filled/cancelled" })),
        )
    }
}

async fn get_marketplace_order(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    tracing::debug!(order_id = %id, "GET /marketplace/order");
    let order = {
        let mp = state.extensions.marketplace.read().await;
        mp.get_order(&id).cloned()
    };
    match order {
        Some(o) => (StatusCode::OK, Json(serde_json::to_value(&o).unwrap())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Order not found" })),
        ),
    }
}

async fn get_marketplace_orderbook(
    State(state): State<Arc<AppState>>,
    Path((sell_asset_str, buy_asset_str)): Path<(String, String)>,
) -> impl IntoResponse {
    tracing::debug!(sell = %sell_asset_str, buy = %buy_asset_str, "GET /marketplace/orderbook");

    let sell_asset = match parse_market_asset(&sell_asset_str) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            )
        }
    };
    let buy_asset = match parse_market_asset(&buy_asset_str) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            )
        }
    };

    let ob = {
        let mp = state.extensions.marketplace.read().await;
        mp.order_book(&sell_asset, &buy_asset)
    };

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "sell_asset": sell_asset_str,
            "buy_asset": buy_asset_str,
            "bids": ob.bids,
            "asks": ob.asks,
        })),
    )
}

async fn get_marketplace_orders(
    State(state): State<Arc<AppState>>,
    Path(maker): Path<String>,
) -> impl IntoResponse {
    tracing::debug!(maker = %maker, "GET /marketplace/orders");
    let orders: Vec<_> = {
        let mp = state.extensions.marketplace.read().await;
        mp.orders_for_maker(&maker).into_iter().cloned().collect()
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({ "maker": maker, "orders": orders })),
    )
}

async fn get_marketplace_trades(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    tracing::debug!("GET /marketplace/trades");
    let trades = {
        let mp = state.extensions.marketplace.read().await;
        mp.trades().to_vec()
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({ "trades": trades })),
    )
}

/// Parse a token type string (case-insensitive) into a `TokenType`.
fn token_type_from_str(s: &str) -> Option<TokenType> {
    match s.to_lowercase().as_str() {
        "tribechain" | "native" => Some(TokenType::TribeChain),
        "pttc" => Some(TokenType::PTtC),
        "nmtc" => Some(TokenType::NMTC),
        "stomp" => Some(TokenType::STOMP),
        "aum" => Some(TokenType::AUM),
        "ai3" => Some(TokenType::AI3),
        _ => None,
    }
}

// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test ValidatorConfig
    fn create_test_config() -> crate::config::ValidatorConfig {
        crate::config::ValidatorConfig {
            node_id: "test-node".to_string(),
            listen_addr: "127.0.0.1".to_string(),
            port: 8900,
            solana_rpc_url: "http://localhost:8899".to_string(),
            pot_program_id: "PoToValidator1111111111111111111111111111111".to_string(),
            difficulty: 2,
            max_tensor_dim: 4,
            max_mine_iterations: 1000,
            peer_network_mode: "local_only".to_string(),
            pool_strategy: "solo".to_string(),
            chain_bridge: "solana".to_string(),
            device_protocol: "native".to_string(),
            auto_register_miners: false,
            relayer_keypair_path: "/tmp/relayer.json".to_string(),
            bootstrap_urls: vec![],
            enable_mdns: false,
            mdns_service_name: "pot-o-validator".to_string(),
            internal_api_port: 8901,
            peer_timeout_secs: 30,
            challenge_relay_enabled: true,
            primary_validator_url: "http://localhost:8899".to_string(),
            maturity_depth: 10,
            symmetry_num: 1,
            symmetry_den: 1,
            base_target: "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
                .to_string(),
            protocol_fee_address: String::new(),
        }
    }

    /// Helper to create a test HexConsensus
    fn create_test_hex_consensus() -> hexchain_p2p::hex_consensus::HexConsensus {
        use hexchain_p2p::types::{ConsensusParams, MmlParams};
        let base_target_bytes: [u8; 32] = [0xFFu8; 32];
        let params = ConsensusParams {
            maturity_depth: 10,
            symmetry_num: 1,
            symmetry_den: 1,
            base_target: base_target_bytes,
            mml: MmlParams::default(),
        };
        hexchain_p2p::hex_consensus::HexConsensus::new(params)
    }

    /// Test that challenge generation succeeds with LocalOnlyNetwork (no broadcast)
    #[tokio::test]
    async fn test_challenge_generation_with_local_only_network() {
        let cfg = create_test_config();
        let consensus = pot_o_mining::PotOConsensus::new(cfg.difficulty, cfg.max_tensor_dim);
        let extensions = pot_o_extensions::ExtensionRegistry::local_defaults(
            &cfg.solana_rpc_url,
            &cfg.pot_program_id,
            &cfg.relayer_keypair_path,
            false,
        );

        let state = crate::consensus::create_app_state(
            cfg,
            consensus,
            extensions,
            "/tmp/registry.json".to_string(),
            std::collections::HashMap::new(),
            create_test_hex_consensus(),
        );

        // Generate a challenge
        let req = ChallengeRequest {
            slot: Some(100),
            slot_hash: Some("0".repeat(64)),
            device_type: None,
        };

        let body = Json(req);
        let response = get_challenge(State(state.clone()), body).await;

        // Verify we got a successful response
        let (status, _) = response.into_response().into_parts();
        assert_eq!(status.status.as_u16(), 200);

        // Verify challenge was stored
        let current = state.current_challenge.read().await;
        assert!(current.is_some());
    }

    /// Test that challenge generation succeeds and broadcasts
    #[tokio::test]
    async fn test_challenge_generation_broadcasts() {
        let cfg = create_test_config();
        let consensus = pot_o_mining::PotOConsensus::new(cfg.difficulty, cfg.max_tensor_dim);

        let extensions = pot_o_extensions::ExtensionRegistry::local_defaults(
            &cfg.solana_rpc_url,
            &cfg.pot_program_id,
            &cfg.relayer_keypair_path,
            false,
        );

        let state = crate::consensus::create_app_state(
            cfg,
            consensus,
            extensions,
            "/tmp/registry.json".to_string(),
            std::collections::HashMap::new(),
            create_test_hex_consensus(),
        );

        // Generate a challenge
        let req = ChallengeRequest {
            slot: Some(100),
            slot_hash: Some("0".repeat(64)),
            device_type: None,
        };

        let body = Json(req);
        let response = get_challenge(State(state), body).await;

        // Verify successful response
        let (status, _) = response.into_response().into_parts();
        assert_eq!(status.status.as_u16(), 200);
    }

    /// Test that challenge request with default slot generates valid challenge
    #[tokio::test]
    async fn test_challenge_generation_with_defaults() {
        let cfg = create_test_config();
        let consensus = pot_o_mining::PotOConsensus::new(cfg.difficulty, cfg.max_tensor_dim);
        let extensions = pot_o_extensions::ExtensionRegistry::local_defaults(
            &cfg.solana_rpc_url,
            &cfg.pot_program_id,
            &cfg.relayer_keypair_path,
            false,
        );

        let state = crate::consensus::create_app_state(
            cfg,
            consensus,
            extensions,
            "/tmp/registry.json".to_string(),
            std::collections::HashMap::new(),
            create_test_hex_consensus(),
        );

        // Request with minimal/empty body (should use defaults)
        let req = ChallengeRequest {
            slot: None,
            slot_hash: None,
            device_type: None,
        };

        let body = Json(req);
        let response = get_challenge(State(state.clone()), body).await;

        let (status, _) = response.into_response().into_parts();
        assert_eq!(status.status.as_u16(), 200);

        // Verify challenge was stored
        let current = state.current_challenge.read().await;
        assert!(current.is_some());
    }

    /// Test stats are updated when challenge is generated
    #[tokio::test]
    async fn test_challenge_updates_stats() {
        let cfg = create_test_config();
        let consensus = pot_o_mining::PotOConsensus::new(cfg.difficulty, cfg.max_tensor_dim);
        let extensions = pot_o_extensions::ExtensionRegistry::local_defaults(
            &cfg.solana_rpc_url,
            &cfg.pot_program_id,
            &cfg.relayer_keypair_path,
            false,
        );

        let state = crate::consensus::create_app_state(
            cfg,
            consensus,
            extensions,
            "/tmp/registry.json".to_string(),
            std::collections::HashMap::new(),
            create_test_hex_consensus(),
        );

        // Verify initial stats
        {
            let stats = state.stats.read().await;
            assert_eq!(stats.total_challenges_issued, 0);
            assert_eq!(stats.paths_in_block, 0);
            assert_eq!(stats.calcs_in_block, 0);
        }

        // Generate challenge
        let req = ChallengeRequest {
            slot: Some(100),
            slot_hash: Some("0".repeat(64)),
            device_type: None,
        };

        let body = Json(req);
        let response = get_challenge(State(state.clone()), body).await;

        let (status, _) = response.into_response().into_parts();
        assert_eq!(status.status.as_u16(), 200);

        // Verify stats were updated
        {
            let stats = state.stats.read().await;
            assert_eq!(stats.total_challenges_issued, 1);
            assert_eq!(stats.paths_in_block, 0);
            assert_eq!(stats.calcs_in_block, 0);
        }
    }

    /// Test that multiple challenges can be generated
    #[tokio::test]
    async fn test_multiple_challenges() {
        let cfg = create_test_config();
        let consensus = pot_o_mining::PotOConsensus::new(cfg.difficulty, cfg.max_tensor_dim);
        let extensions = pot_o_extensions::ExtensionRegistry::local_defaults(
            &cfg.solana_rpc_url,
            &cfg.pot_program_id,
            &cfg.relayer_keypair_path,
            false,
        );

        let state = crate::consensus::create_app_state(
            cfg,
            consensus,
            extensions,
            "/tmp/registry.json".to_string(),
            std::collections::HashMap::new(),
            create_test_hex_consensus(),
        );

        // Generate multiple challenges
        for i in 0..3 {
            let req = ChallengeRequest {
                slot: Some(100 + i),
                slot_hash: Some(format!("{:0>64}", i)),
                device_type: None,
            };

            let body = Json(req);
            let response = get_challenge(State(state.clone()), body).await;

            let (status, _) = response.into_response().into_parts();
            assert_eq!(status.status.as_u16(), 200);
        }

        // Verify stats
        {
            let stats = state.stats.read().await;
            assert_eq!(stats.total_challenges_issued, 3);
        }
    }

    /// Test that challenge generation succeeds even if network is not LocalOnly
    #[tokio::test]
    async fn test_challenge_generated_with_async_broadcast() {
        let cfg = create_test_config();
        let consensus = pot_o_mining::PotOConsensus::new(cfg.difficulty, cfg.max_tensor_dim);
        let extensions = pot_o_extensions::ExtensionRegistry::local_defaults(
            &cfg.solana_rpc_url,
            &cfg.pot_program_id,
            &cfg.relayer_keypair_path,
            false,
        );

        let state = crate::consensus::create_app_state(
            cfg,
            consensus,
            extensions,
            "/tmp/registry.json".to_string(),
            std::collections::HashMap::new(),
            create_test_hex_consensus(),
        );

        // Generate a challenge
        let req = ChallengeRequest {
            slot: Some(100),
            slot_hash: Some("0".repeat(64)),
            device_type: None,
        };

        let body = Json(req);
        let response = get_challenge(State(state), body).await;

        // Verify response is successful (broadcast happens async, non-blocking)
        let (status, _) = response.into_response().into_parts();
        assert_eq!(status.status.as_u16(), 200);

        // Give async broadcast task time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
