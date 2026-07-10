use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use hexchain_p2p::hex_consensus::HexProof;
use hexchain_p2p::lattice_geometry::HCPCoord;
use hexchain_p2p::lattice_store::LatticeSnapshot;
use serde::Deserialize;

use crate::consensus::{accept_block, rollback_ledger_to, AppState, RELAYED_BLOCKS_MAX};
use crate::rate_limit::{rate_limit_middleware, RateLimiter};

type HexApiResponse = (StatusCode, Json<serde_json::Value>);

pub fn hex_routes(state: Arc<AppState>) -> Router {
    let hex_submit_limiter = RateLimiter::new(3, 1);

    let hex_submit_route = Router::new()
        .route("/hexchain/submit", post(post_hex_submit))
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(axum::Extension(hex_submit_limiter));

    Router::new()
        .route("/hexchain/challenge", post(post_hex_challenge))
        .route("/hexchain/status", get(get_hex_status))
        .route("/hexchain/lattice", get(get_hex_lattice_all))
        .route("/hexchain/lattice/sync", post(post_hex_lattice_sync))
        .route("/hexchain/lattice/{q}/{r}/{s}", get(get_hex_lattice_coord))
        .route("/hexchain/block/{height}", get(get_hex_block_by_height))
        .route("/hexchain/blocks", get(get_hex_blocks_batch))
        .merge(hex_submit_route)
        .with_state(state)
}

#[derive(Deserialize)]
struct HexChallengeRequest {
    slot: u64,
    slot_hash: String,
}

async fn post_hex_challenge(
    State(state): State<Arc<AppState>>,
    Json(body): Json<HexChallengeRequest>,
) -> impl IntoResponse {
    tracing::debug!(slot = body.slot, "POST /hexchain/challenge");
    let challenge = state
        .hex_consensus
        .generate_challenge(body.slot, &body.slot_hash);
    {
        let mut current = state.hex_current_challenge.write().await;
        *current = Some(challenge.clone());
    }
    tracing::info!(
        challenge_id = %challenge.id,
        coord = ?challenge.coord,
        "POST /hexchain/challenge issued"
    );
    (
        StatusCode::OK,
        Json(serde_json::to_value(&challenge).unwrap_or_default()),
    )
}

async fn post_hex_submit(
    State(state): State<Arc<AppState>>,
    Json(proof): Json<HexProof>,
) -> impl IntoResponse {
    tracing::debug!(challenge_id = %proof.challenge_id, "POST /hexchain/submit");

    match state.hex_consensus.verify_proof(&proof) {
        Ok(true) => match state.hex_consensus.submit_block(&proof) {
            Ok(depth) => {
                if state.extensions.tribechain_enabled {
                    let mempool = state.extensions.mempool.as_deref();
                    let block_store = state.extensions.block_store.as_deref();
                    let mut ledger = state.extensions.ledger.write().await;
                    if let Err(e) = accept_block(&proof.block, &mut ledger, mempool, block_store) {
                        tracing::warn!(error = %e, "Tribechain block acceptance failed");
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(
                                serde_json::json!({ "accepted": false, "error": "internal error" }),
                            ),
                        );
                    }
                    *state.canonical_tip_height.write().await += 1;
                    drop(ledger);
                    relay_block(&state, &proof).await;
                }
                // Persist lattice immediately so restarts never lose accepted blocks
                if let Err(e) = state.hex_consensus.store.save_to_file() {
                    tracing::warn!(error = %e, "Hex lattice post-submit persist failed (non-fatal)");
                }
                tracing::info!(
                    challenge_id = %proof.challenge_id,
                    coord = ?proof.block.coord,
                    depth = depth,
                    "POST /hexchain/submit accepted"
                );
                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "accepted": true,
                        "depth": depth,
                        "block_hash": hex::encode(proof.block.pow_hash()),
                    })),
                )
            }
            Err(e) => {
                tracing::warn!(error = ?e, "POST /hexchain/submit block insertion failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "accepted": false, "error": "internal error" })),
                )
            }
        },
        Ok(false) => {
            tracing::info!(challenge_id = %proof.challenge_id, "POST /hexchain/submit rejected (genesis mode, no validation)");
            match state.hex_consensus.submit_block(&proof) {
                Ok(depth) => {
                    if state.extensions.tribechain_enabled {
                        let mempool = state.extensions.mempool.as_deref();
                        let block_store = state.extensions.block_store.as_deref();
                        let mut ledger = state.extensions.ledger.write().await;
                        if let Err(e) =
                            accept_block(&proof.block, &mut ledger, mempool, block_store)
                        {
                            tracing::warn!(error = %e, "Tribechain genesis block acceptance failed");
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(
                                    serde_json::json!({ "accepted": false, "error": "internal error" }),
                                ),
                            );
                        }
                        *state.canonical_tip_height.write().await += 1;
                        drop(ledger);
                        relay_block(&state, &proof).await;
                    }
                    // Persist lattice immediately (genesis mode)
                    if let Err(e) = state.hex_consensus.store.save_to_file() {
                        tracing::warn!(error = %e, "Hex lattice genesis persist failed (non-fatal)");
                    }
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "accepted": true,
                            "depth": depth,
                            "block_hash": hex::encode(proof.block.pow_hash()),
                        })),
                    )
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "POST /hexchain/submit genesis block insertion failed");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "accepted": false, "error": "internal error"
                        })),
                    )
                }
            }
        }
        Err(e) => {
            tracing::warn!(error = ?e, "POST /hexchain/submit verification failed");
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "accepted": false, "error": "verification failed" })),
            )
        }
    }
}

async fn get_hex_status(State(state): State<Arc<AppState>>) -> HexApiResponse {
    let occupied = state.hex_consensus.store.all_coords().len();
    let all_blocks = state.hex_consensus.store.all_blocks();
    let latest_depth = all_blocks
        .iter()
        .filter_map(|(_, h)| state.hex_consensus.store.depth_of(h))
        .max();
    let current_challenge = state.hex_current_challenge.read().await.clone();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "occupied_coords": occupied,
            "latest_depth": latest_depth,
            "current_challenge": current_challenge,
        })),
    )
}

async fn get_hex_lattice_all(State(state): State<Arc<AppState>>) -> HexApiResponse {
    let coords: Vec<HCPCoord> = state.hex_consensus.store.all_coords().into_iter().collect();
    let blocks: Vec<serde_json::Value> = coords
        .iter()
        .filter_map(|c| {
            let hash = state.hex_consensus.store.hash_at(*c)?;
            let depth = state.hex_consensus.store.depth_of(&hash)?;
            Some(serde_json::json!({
                "coord": c,
                "block_hash": hex::encode(hash),
                "depth": depth,
            }))
        })
        .collect();
    (
        StatusCode::OK,
        Json(serde_json::json!({ "blocks": blocks })),
    )
}

async fn post_hex_lattice_sync(
    State(state): State<Arc<AppState>>,
    Json(snapshot): Json<LatticeSnapshot>,
) -> HexApiResponse {
    let before = state.hex_consensus.store.all_coords().len();
    state.hex_consensus.store.merge_snapshot(&snapshot);
    let after = state.hex_consensus.store.all_coords().len();
    let merged = after.saturating_sub(before);
    if let Err(e) = state.hex_consensus.store.save_to_file() {
        tracing::warn!(error = %e, "POST /hexchain/lattice/sync persist failed");
    }

    if state.extensions.tribechain_enabled {
        let new_max_depth = state
            .hex_consensus
            .store
            .all_blocks()
            .iter()
            .filter_map(|(_, h)| state.hex_consensus.store.depth_of(h))
            .max()
            .unwrap_or(0) as i64;
        let canonical_height = *state.canonical_tip_height.read().await as i64;
        if new_max_depth < canonical_height {
            tracing::warn!(
                old_height = canonical_height,
                new_height = new_max_depth,
                "Chain reorg detected, rolling back ledger"
            );
            if let Some(block_store) = &state.extensions.block_store {
                let mut ledger = state.extensions.ledger.write().await;
                let mut tip_height = state.canonical_tip_height.write().await;
                if let Err(e) = rollback_ledger_to(
                    &mut ledger,
                    block_store.as_ref(),
                    new_max_depth as u64,
                    &mut tip_height,
                ) {
                    tracing::error!(error = %e, "Ledger rollback failed");
                }
            }
        }
    }

    tracing::info!(merged, total = after, "POST /hexchain/lattice/sync");
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "accepted": true,
            "merged": merged,
            "total_coords": after,
        })),
    )
}

async fn get_hex_lattice_coord(
    State(state): State<Arc<AppState>>,
    Path((q, r, s)): Path<(i32, i32, i32)>,
) -> HexApiResponse {
    let coord = HCPCoord { q, r, s };
    match state.hex_consensus.store.hash_at(coord) {
        Some(hash) => {
            let depth = state.hex_consensus.store.depth_of(&hash);
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "coord": coord,
                    "block_hash": hex::encode(hash),
                    "depth": depth,
                })),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "No block at this coordinate" })),
        ),
    }
}

async fn get_hex_block_by_height(
    State(state): State<Arc<AppState>>,
    Path(height): Path<u64>,
) -> HexApiResponse {
    let block_store = match &state.extensions.block_store {
        Some(bs) => bs,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "block store not available"})),
            )
        }
    };

    let stored = match block_store.at_height(height) {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "No block at this height"})),
            )
        }
    };

    let block: hexchain_p2p::block::HexBlock = match serde_json::from_str(&stored.block_json) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse stored block");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal error"})),
            );
        }
    };

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "height": stored.height,
            "hash": hex::encode(stored.hash),
            "block": block,
        })),
    )
}

#[derive(Deserialize)]
struct BlocksBatchQuery {
    from_height: Option<u64>,
    limit: Option<usize>,
}

async fn get_hex_blocks_batch(
    State(state): State<Arc<AppState>>,
    Query(query): Query<BlocksBatchQuery>,
) -> HexApiResponse {
    let block_store = match &state.extensions.block_store {
        Some(bs) => bs,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "block store not available"})),
            )
        }
    };

    let from_height = query.from_height.unwrap_or(0);
    let limit = query.limit.unwrap_or(50).min(200);
    let latest = block_store.latest_height();

    let mut blocks = Vec::new();
    for h in from_height..=latest {
        if blocks.len() >= limit {
            break;
        }
        if let Some(stored) = block_store.at_height(h) {
            blocks.push(serde_json::json!({
                "height": stored.height,
                "hash": hex::encode(stored.hash),
                "block_json": stored.block_json,
            }));
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "blocks": blocks,
            "count": blocks.len(),
        })),
    )
}

async fn relay_block(state: &AppState, proof: &HexProof) {
    let block_hash = proof.block.pow_hash();
    {
        let mut relayed = state.relayed_blocks.write().await;
        if relayed.contains(&block_hash) {
            return;
        }
        relayed.push_back(block_hash);
        while relayed.len() > RELAYED_BLOCKS_MAX {
            relayed.pop_front();
        }
    }

    let network = state.extensions.network.clone();
    let proof_json = match serde_json::to_value(proof) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to serialize block for relay");
            return;
        }
    };
    let block_height = proof.block.height;

    tokio::spawn(async move {
        match network.broadcast_block(&proof_json).await {
            Ok(n) if n > 0 => {
                tracing::info!(peers = n, height = block_height, "Block relayed to peers");
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(error = %e, "Block relay to peers failed");
            }
        }
    });
}
