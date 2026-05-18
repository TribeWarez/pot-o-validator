use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use hexchain_p2p::hex_consensus::HexProof;
use hexchain_p2p::lattice_geometry::HCPCoord;
use serde::Deserialize;

use crate::consensus::AppState;

type HexApiResponse = (StatusCode, Json<serde_json::Value>);

pub fn hex_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/hexchain/challenge", post(post_hex_challenge))
        .route("/hexchain/submit", post(post_hex_submit))
        .route("/hexchain/status", get(get_hex_status))
        .route("/hexchain/lattice", get(get_hex_lattice_all))
        .route("/hexchain/lattice/{q}/{r}/{s}", get(get_hex_lattice_coord))
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
    let challenge = state.hex_consensus.generate_challenge(body.slot, &body.slot_hash);
    {
        let mut current = state.hex_current_challenge.write().await;
        *current = Some(challenge.clone());
    }
    tracing::info!(
        challenge_id = %challenge.id,
        coord = ?challenge.coord,
        "POST /hexchain/challenge issued"
    );
    (StatusCode::OK, Json(serde_json::to_value(&challenge).unwrap()))
}

async fn post_hex_submit(
    State(state): State<Arc<AppState>>,
    Json(proof): Json<HexProof>,
) -> impl IntoResponse {
    tracing::debug!(challenge_id = %proof.challenge_id, "POST /hexchain/submit");

    match state.hex_consensus.verify_proof(&proof) {
        Ok(true) => {
            match state.hex_consensus.submit_block(&proof) {
                Ok(depth) => {
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
                        Json(serde_json::json!({ "accepted": false, "error": format!("{:?}", e) })),
                    )
                }
            }
        }
        Ok(false) => {
            tracing::info!(challenge_id = %proof.challenge_id, "POST /hexchain/submit rejected (genesis mode, no validation)");
            match state.hex_consensus.submit_block(&proof) {
                Ok(depth) => {
                    (StatusCode::OK, Json(serde_json::json!({
                        "accepted": true,
                        "depth": depth,
                        "block_hash": hex::encode(proof.block.pow_hash()),
                    })))
                }
                Err(e) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                        "accepted": false, "error": format!("{:?}", e)
                    })))
                }
            }
        }
        Err(e) => {
            tracing::warn!(error = ?e, "POST /hexchain/submit verification failed");
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "accepted": false, "error": format!("{:?}", e) })),
            )
        }
    }
}

async fn get_hex_status(State(state): State<Arc<AppState>>) -> HexApiResponse {
    let occupied = state.hex_consensus.store.all_coords().len();
    let all_blocks = state.hex_consensus.store.all_blocks();
    let latest_depth = all_blocks.iter().filter_map(|(_, h)| state.hex_consensus.store.depth_of(h)).max();
    let current_challenge = state.hex_current_challenge.read().await.clone();

    (StatusCode::OK, Json(serde_json::json!({
        "occupied_coords": occupied,
        "latest_depth": latest_depth,
        "current_challenge": current_challenge,
    })))
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
    (StatusCode::OK, Json(serde_json::json!({ "blocks": blocks })))
}

async fn get_hex_lattice_coord(
    State(state): State<Arc<AppState>>,
    Path((q, r, s)): Path<(i32, i32, i32)>,
) -> HexApiResponse {
    let coord = HCPCoord { q, r, s };
    match state.hex_consensus.store.hash_at(coord) {
        Some(hash) => {
            let depth = state.hex_consensus.store.depth_of(&hash);
            (StatusCode::OK, Json(serde_json::json!({
                "coord": coord,
                "block_hash": hex::encode(hash),
                "depth": depth,
            })))
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "No block at this coordinate" })),
        ),
    }
}
