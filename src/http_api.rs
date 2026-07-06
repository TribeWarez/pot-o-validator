use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{delete, get, post},
    Json, Router,
};
use pot_o_core::TokenType;
use pot_o_extensions::marketplace::{parse_market_asset, OrderSide};
use pot_o_extensions::{calculate_mining_reward, ProofRecord};
use pot_o_mining::{PotOProof, ProofPayload};
use serde::Deserialize;

use crate::consensus::AppState;
use crate::device_registry::{
    normalize_device_type, prune_stale_devices, spawn_persist_registry, CurrentCalculation,
    RegisteredDevice, DEVICE_TYPE_KEYS,
};

async fn auth_middleware(
    state: &Arc<AppState>,
    bearer: Option<&str>,
) -> Result<String, StatusCode> {
    let token = bearer
        .and_then(|b| b.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    state
        .auth
        .validate_token(token)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)
}

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(|| async { Redirect::permanent("/status") }))
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/challenge", post(get_challenge))
        .route("/submit", post(submit_proof))
        .route("/miners/:pubkey", get(get_miner))
        .route("/pool", get(pool_info))
        .route("/devices/register", post(register_device))
        .route("/devices/progress", post(device_progress))
        .route("/devices", get(get_devices))
        .route("/network/peers", get(get_peers))
        .route(
            "/token/balance/:address/:token_type",
            get(get_token_balance),
        )
        .route("/token/transfer", post(post_token_transfer))
        .route("/token/tx/:address", get(get_token_tx_history))
        .route("/token/tribe/supply", get(get_tribe_supply))
        .route("/api/tx", post(post_tribechain_tx))
        .route("/api/nonce/:address", get(get_tribechain_nonce))
        .route("/api/blocks", get(get_tribechain_blocks))
        .route("/marketplace/order", post(post_marketplace_order))
        .route("/marketplace/order/{id}", delete(delete_marketplace_order))
        .route("/marketplace/order/{id}", get(get_marketplace_order))
        .route(
            "/marketplace/orderbook/{sell_asset}/{buy_asset}",
            get(get_marketplace_orderbook),
        )
        .route("/marketplace/orders/{maker}", get(get_marketplace_orders))
        .route("/marketplace/trades", get(get_marketplace_trades))
        .route("/ws", get(ws_handler))
        .route("/auth/challenge", post(post_auth_challenge))
        .route("/auth/verify", post(post_auth_verify))
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
    let peers = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        state.extensions.network.discover_peers(),
    )
    .await
    .ok()
    .and_then(|r| r.ok())
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
                        tracing::debug!(challenge_id = %challenge_clone.id, "Challenge broadcast to peers completed");
                    }
                    Err(e) => {
                        tracing::warn!(challenge_id = %challenge_clone.id, error = %e, "Challenge broadcast to peers failed (non-fatal)");
                    }
                }
            });

            let state_clone = state.clone();
            let challenge_json = serde_json::to_string(&challenge).unwrap_or_default();
            tokio::spawn(async move {
                state_clone
                    .extensions
                    .messaging
                    .broadcast(&pot_o_extensions::ValidatorMessage::Challenge { challenge_json })
                    .await;
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
    device_id: Option<String>,
    device_type: Option<String>,
}

async fn submit_proof(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<SubmitRequest>,
) -> impl IntoResponse {
    if let Err(status) = auth_middleware(
        &state,
        headers.get("authorization").and_then(|v| v.to_str().ok()),
    )
    .await
    {
        return (
            status,
            Json(serde_json::json!({ "accepted": false, "error": "unauthorized" })),
        );
    }

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
                    prune_stale_devices(&mut reg);
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

                let path_distance = body.proof.path_distance;
                let reward_amount = calculate_mining_reward(chal.difficulty, path_distance);
                let proof_record = ProofRecord {
                    miner_pubkey: body.proof.miner_pubkey.clone(),
                    challenge_id: body.proof.challenge_id.clone(),
                    reward: reward_amount,
                    timestamp: chrono::Utc::now(),
                };
                if let Ok(shares) = state
                    .extensions
                    .pool
                    .calculate_shares(&[proof_record], reward_amount)
                {
                    {
                        let mut ledger = state.extensions.ledger.write().await;
                        for share in &shares {
                            ledger.issue(
                                &share.miner_pubkey,
                                &TokenType::TribeChain,
                                share.reward_amount,
                            );
                        }
                    }
                    let mut stats = state.stats.write().await;
                    stats.total_tribe_minted =
                        stats.total_tribe_minted.saturating_add(reward_amount);
                    stats.total_rewards_paid = stats.total_rewards_paid.saturating_add(1);
                    tracing::debug!(
                        miner = %body.proof.miner_pubkey,
                        reward = reward_amount,
                        shares = shares.len(),
                        "TRIBE mining reward distributed"
                    );
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
                            "POST /submit accepted"
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
    let balance = state
        .extensions
        .ledger
        .read()
        .await
        .balance_of(&pubkey, &TokenType::TribeChain);
    let nonce = state.extensions.ledger.read().await.current_nonce(&pubkey);
    tracing::debug!(pubkey = %pubkey, balance, "GET /miners/:pubkey");
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "pubkey": pubkey,
            "balance": balance,
            "nonce": nonce,
        })),
    )
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
    miner_pubkey: Option<String>,
}

async fn register_device(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<DeviceRegisterRequest>,
) -> impl IntoResponse {
    if let Err(status) = auth_middleware(
        &state,
        headers.get("authorization").and_then(|v| v.to_str().ok()),
    )
    .await
    {
        return (
            status,
            Json(serde_json::json!({ "registered": false, "error": "unauthorized" })),
        );
    }

    let device_id = body
        .device_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let device_type_normalized = normalize_device_type(&body.device_type);

    let our_type = state.extensions.device.device_type();
    let our_type_str = format!("{:?}", our_type).to_lowercase();
    if device_type_normalized != our_type_str && body.device_type != "any" {
        tracing::warn!(
            requested = %device_type_normalized,
            configured = %our_type_str,
            "Device type mismatch — requested device type differs from configured protocol"
        );
    }
    let now = chrono::Utc::now();
    let is_new = {
        let mut reg = state.device_registry.write().await;
        prune_stale_devices(&mut reg);
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
        match state.extensions.chain.register_miner(miner_pubkey).await {
            Ok(_) => {
                tracing::info!(
                    device_id = %device_id,
                    miner_pubkey = %miner_pubkey,
                    "Registered miner at device registration"
                );
                true
            }
            Err(e) => {
                tracing::warn!(
                    device_id = %device_id,
                    miner_pubkey = %miner_pubkey,
                    error = %e,
                    "Register miner at registration failed"
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
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "registered": true,
            "device_type": body.device_type,
            "device_id": device_id,
            "miner_registered": miner_registered,
        })),
    )
}

#[derive(Deserialize)]
struct DeviceProgressRequest {
    device_id: Option<String>,
    miner_pubkey: Option<String>,
    device_type: Option<String>,
    challenge_id: String,
    hash: String,
}

async fn device_progress(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<DeviceProgressRequest>,
) -> impl IntoResponse {
    if let Err(status) = auth_middleware(
        &state,
        headers.get("authorization").and_then(|v| v.to_str().ok()),
    )
    .await
    {
        return (
            status,
            Json(serde_json::json!({ "ok": false, "error": "unauthorized" })),
        );
    }

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
        prune_stale_devices(&mut reg);
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
    {
        let mut reg = state.device_registry.write().await;
        prune_stale_devices(&mut reg);
    }
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
    headers: axum::http::HeaderMap,
    Json(body): Json<TransferRequest>,
) -> impl IntoResponse {
    if let Err(status) = auth_middleware(
        &state,
        headers.get("authorization").and_then(|v| v.to_str().ok()),
    )
    .await
    {
        return (status, Json(serde_json::json!({ "error": "unauthorized" })));
    }

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

async fn get_tribe_supply(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    tracing::debug!("GET /token/tribe/supply");
    let supply = state
        .extensions
        .ledger
        .read()
        .await
        .total_supply(&TokenType::TribeChain);
    let minted = {
        let stats = state.stats.read().await;
        stats.total_tribe_minted
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "supply": supply,
            "total_minted": minted,
            "token": "TribeChain",
        })),
    )
}

// ---------------------------------------------------------------------------
// Tribechain public API handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TribechainTxRequest {
    tx: pot_o_extensions::tx::TransferTransaction,
}

async fn post_tribechain_tx(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TribechainTxRequest>,
) -> impl IntoResponse {
    tracing::info!("POST /api/tx received");
    tracing::debug!(from = %body.tx.from, "POST /api/tx");

    if !state.extensions.tribechain_enabled {
        tracing::warn!("tribechain not enabled");
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "tribechain not enabled"})),
        );
    }

    let Some(mempool) = &state.extensions.mempool else {
        tracing::warn!("mempool not available");
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "mempool not available"})),
        );
    };

    if body.tx.signature.len() != 64 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid signature length"})),
        );
    }

    if body.tx.tx_hash.len() != 32 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid tx_hash length"})),
        );
    }

    match mempool
        .submit(body.tx.clone(), &state.extensions.ledger)
        .await
    {
        Ok(tx_hash) => {
            let tx_json = serde_json::to_value(&body.tx).unwrap_or_default();
            let _ = state
                .extensions
                .network
                .broadcast_transaction(&tx_json)
                .await;
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "accepted": true,
                    "tx_hash": hex::encode(tx_hash),
                })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

async fn get_tribechain_nonce(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    tracing::debug!(address = %address, "GET /api/nonce");
    let nonce = state.extensions.ledger.read().await.current_nonce(&address);
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "address": address,
            "nonce": nonce,
        })),
    )
}

#[derive(Debug, Deserialize)]
struct BlocksQuery {
    from_height: Option<u64>,
    limit: Option<usize>,
}

async fn get_tribechain_blocks(
    State(state): State<Arc<AppState>>,
    Query(query): Query<BlocksQuery>,
) -> impl IntoResponse {
    tracing::debug!("GET /api/blocks");
    let Some(block_store) = &state.extensions.block_store else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "block store not available"})),
        );
    };

    let from_height = query.from_height.unwrap_or(0);
    let limit = query.limit.unwrap_or(100).min(1000);
    let latest = block_store.latest_height();

    let mut blocks = Vec::new();
    for h in from_height..=latest {
        if blocks.len() >= limit {
            break;
        }
        if let Some(stored) = block_store.at_height(h) {
            if let Ok(block) = serde_json::from_value::<hexchain_p2p::block::HexBlock>(
                serde_json::from_str(&stored.block_json).unwrap_or_default(),
            ) {
                blocks.push(serde_json::json!({
                    "height": block.height,
                    "hash": hex::encode(stored.hash),
                    "coord": block.coord,
                    "tx_count": block.transactions.as_ref().map(|t| t.len()).unwrap_or(0),
                    "timestamp": block.timestamp,
                }));
            }
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "blocks": blocks,
            "latest_height": latest,
        })),
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
    headers: axum::http::HeaderMap,
    Json(body): Json<PlaceOrderRequest>,
) -> impl IntoResponse {
    if let Err(status) = auth_middleware(
        &state,
        headers.get("authorization").and_then(|v| v.to_str().ok()),
    )
    .await
    {
        return (status, Json(serde_json::json!({ "error": "unauthorized" })));
    }

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
// Auth handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct AuthChallengeRequest {
    pubkey: String,
}

async fn post_auth_challenge(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AuthChallengeRequest>,
) -> impl IntoResponse {
    let nonce = state.auth.create_challenge(&body.pubkey).await;
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "pubkey": body.pubkey,
            "nonce": hex::encode(nonce),
        })),
    )
}

#[derive(Deserialize)]
struct AuthVerifyRequest {
    pubkey: String,
    signature: String,
    message: String,
}

async fn post_auth_verify(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AuthVerifyRequest>,
) -> impl IntoResponse {
    let signature = match hex::decode(&body.signature) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Invalid signature hex: {}", e) })),
            )
        }
    };
    let message = match hex::decode(&body.message) {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Invalid message hex: {}", e) })),
            )
        }
    };

    match state
        .auth
        .verify_challenge(&body.pubkey, &signature, &message)
        .await
    {
        Ok(token) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "token": token,
                "pubkey": body.pubkey,
            })),
        ),
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

// ---------------------------------------------------------------------------
// WebSocket — Messaging Protocol
// ---------------------------------------------------------------------------

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    tracing::debug!("WS upgrade requested");
    ws.on_upgrade(move |socket| handle_ws_socket(socket, state))
}

async fn handle_ws_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::unbounded_channel();

    let mut device_id: Option<String> = None;

    loop {
        tokio::select! {
            ws_msg = socket.recv() => {
                match ws_msg {
                    Some(Ok(Message::Text(text)))
                        if handle_ws_text(
                            &text, &state, &msg_tx, &mut device_id,
                        ).await.is_err() => {
                        break;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            json = msg_rx.recv() => {
                match json {
                    Some(json) => {
                        if socket.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    if let Some(did) = device_id {
        state.extensions.messaging.unregister(&did).await;
    }
}

async fn handle_ws_text(
    text: &str,
    state: &Arc<AppState>,
    msg_tx: &tokio::sync::mpsc::UnboundedSender<String>,
    device_id: &mut Option<String>,
) -> Result<(), ()> {
    match serde_json::from_str::<pot_o_extensions::MinerMessage>(text) {
        Ok(miner_msg) => match miner_msg {
            pot_o_extensions::MinerMessage::Subscribe {
                device_id: did,
                device_type: _,
            } => {
                let did_clone = did.clone();
                state
                    .extensions
                    .messaging
                    .register(did.clone(), msg_tx.clone())
                    .await;
                *device_id = Some(did_clone.clone());
                let _ = msg_tx.send(
                    pot_o_extensions::ValidatorMessage::Subscribed {
                        device_id: did_clone,
                    }
                    .to_json(),
                );
                Ok(())
            }
            pot_o_extensions::MinerMessage::Unsubscribe { device_id: did } => {
                state.extensions.messaging.unregister(&did).await;
                *device_id = None;
                Err(())
            }
            pot_o_extensions::MinerMessage::Heartbeat { device_id: _ } => {
                let _ = msg_tx.send(pot_o_extensions::ValidatorMessage::HeartbeatAck.to_json());
                Ok(())
            }
            pot_o_extensions::MinerMessage::SubmitProof { .. } => {
                let _ = msg_tx.send(
                    pot_o_extensions::ValidatorMessage::Error {
                        code: "use_http_submit".into(),
                        message: "Use POST /submit for proof submission".into(),
                    }
                    .to_json(),
                );
                Ok(())
            }
            pot_o_extensions::MinerMessage::Progress { .. } => Ok(()),
        },
        Err(e) => {
            let _ = msg_tx.send(
                pot_o_extensions::ValidatorMessage::Error {
                    code: "parse_error".into(),
                    message: format!("Invalid message: {e}"),
                }
                .to_json(),
            );
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> crate::config::ValidatorConfig {
        crate::config::ValidatorConfig {
            node_id: "test-node".to_string(),
            listen_addr: "127.0.0.1".to_string(),
            port: 8900,
            pot_program_id: String::new(),
            difficulty: 2,
            max_tensor_dim: 4,
            max_mine_iterations: 1000,
            peer_network_mode: "local_only".to_string(),
            pool_strategy: "solo".to_string(),
            device_protocol: "native".to_string(),
            bootstrap_urls: vec![],
            enable_mdns: false,
            mdns_service_name: "pot-o-validator".to_string(),
            internal_api_port: 8901,
            peer_timeout_secs: 30,
            challenge_relay_enabled: true,
            maturity_depth: 10,
            symmetry_num: 1,
            symmetry_den: 1,
            base_target: "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
                .to_string(),
            protocol_fee_address: String::new(),
            marketplace_fee_bps: 25,
            tribechain_enabled: false,
            tribechain_min_fee: 0,
            tribechain_max_pool_size: 10_000,
            tribechain_max_txs_per_block: 1000,
            tribechain_genesis_path: String::new(),
            tribechain_miner_address: String::new(),
            tribechain_blockstore_path: "blockstore.json".to_string(),
        }
    }

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

    #[tokio::test]
    async fn test_challenge_generation_with_local_only_network() {
        let cfg = create_test_config();
        let consensus = pot_o_mining::PotOConsensus::new(cfg.difficulty, cfg.max_tensor_dim);
        let extensions = pot_o_extensions::ExtensionRegistry::local_defaults("", 25);

        let state = crate::consensus::create_app_state(
            cfg,
            consensus,
            extensions,
            "/tmp/registry.json".to_string(),
            std::collections::HashMap::new(),
            create_test_hex_consensus(),
        );

        let req = ChallengeRequest {
            slot: Some(100),
            slot_hash: Some("0".repeat(64)),
            device_type: None,
        };

        let body = Json(req);
        let response = get_challenge(State(state.clone()), body).await;

        let (status, _) = response.into_response().into_parts();
        assert_eq!(status.status.as_u16(), 200);

        let current = state.current_challenge.read().await;
        assert!(current.is_some());
    }

    #[tokio::test]
    async fn test_challenge_generation_broadcasts() {
        let cfg = create_test_config();
        let consensus = pot_o_mining::PotOConsensus::new(cfg.difficulty, cfg.max_tensor_dim);
        let extensions = pot_o_extensions::ExtensionRegistry::local_defaults("", 25);

        let state = crate::consensus::create_app_state(
            cfg,
            consensus,
            extensions,
            "/tmp/registry.json".to_string(),
            std::collections::HashMap::new(),
            create_test_hex_consensus(),
        );

        let req = ChallengeRequest {
            slot: Some(100),
            slot_hash: Some("0".repeat(64)),
            device_type: None,
        };

        let body = Json(req);
        let response = get_challenge(State(state), body).await;

        let (status, _) = response.into_response().into_parts();
        assert_eq!(status.status.as_u16(), 200);
    }

    #[tokio::test]
    async fn test_challenge_generation_with_defaults() {
        let cfg = create_test_config();
        let consensus = pot_o_mining::PotOConsensus::new(cfg.difficulty, cfg.max_tensor_dim);
        let extensions = pot_o_extensions::ExtensionRegistry::local_defaults("", 25);

        let state = crate::consensus::create_app_state(
            cfg,
            consensus,
            extensions,
            "/tmp/registry.json".to_string(),
            std::collections::HashMap::new(),
            create_test_hex_consensus(),
        );

        let req = ChallengeRequest {
            slot: None,
            slot_hash: None,
            device_type: None,
        };

        let body = Json(req);
        let response = get_challenge(State(state.clone()), body).await;

        let (status, _) = response.into_response().into_parts();
        assert_eq!(status.status.as_u16(), 200);

        let current = state.current_challenge.read().await;
        assert!(current.is_some());
    }

    #[tokio::test]
    async fn test_challenge_updates_stats() {
        let cfg = create_test_config();
        let consensus = pot_o_mining::PotOConsensus::new(cfg.difficulty, cfg.max_tensor_dim);
        let extensions = pot_o_extensions::ExtensionRegistry::local_defaults("", 25);

        let state = crate::consensus::create_app_state(
            cfg,
            consensus,
            extensions,
            "/tmp/registry.json".to_string(),
            std::collections::HashMap::new(),
            create_test_hex_consensus(),
        );

        {
            let stats = state.stats.read().await;
            assert_eq!(stats.total_challenges_issued, 0);
            assert_eq!(stats.paths_in_block, 0);
            assert_eq!(stats.calcs_in_block, 0);
        }

        let req = ChallengeRequest {
            slot: Some(100),
            slot_hash: Some("0".repeat(64)),
            device_type: None,
        };

        let body = Json(req);
        let response = get_challenge(State(state.clone()), body).await;

        let (status, _) = response.into_response().into_parts();
        assert_eq!(status.status.as_u16(), 200);

        {
            let stats = state.stats.read().await;
            assert_eq!(stats.total_challenges_issued, 1);
            assert_eq!(stats.paths_in_block, 0);
            assert_eq!(stats.calcs_in_block, 0);
        }
    }

    #[tokio::test]
    async fn test_multiple_challenges() {
        let cfg = create_test_config();
        let consensus = pot_o_mining::PotOConsensus::new(cfg.difficulty, cfg.max_tensor_dim);
        let extensions = pot_o_extensions::ExtensionRegistry::local_defaults("", 25);

        let state = crate::consensus::create_app_state(
            cfg,
            consensus,
            extensions,
            "/tmp/registry.json".to_string(),
            std::collections::HashMap::new(),
            create_test_hex_consensus(),
        );

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

        {
            let stats = state.stats.read().await;
            assert_eq!(stats.total_challenges_issued, 3);
        }
    }

    #[tokio::test]
    async fn test_challenge_generated_with_async_broadcast() {
        let cfg = create_test_config();
        let consensus = pot_o_mining::PotOConsensus::new(cfg.difficulty, cfg.max_tensor_dim);
        let extensions = pot_o_extensions::ExtensionRegistry::local_defaults("", 25);

        let state = crate::consensus::create_app_state(
            cfg,
            consensus,
            extensions,
            "/tmp/registry.json".to_string(),
            std::collections::HashMap::new(),
            create_test_hex_consensus(),
        );

        let req = ChallengeRequest {
            slot: Some(100),
            slot_hash: Some("0".repeat(64)),
            device_type: None,
        };

        let body = Json(req);
        let response = get_challenge(State(state), body).await;

        let (status, _) = response.into_response().into_parts();
        assert_eq!(status.status.as_u16(), 200);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
