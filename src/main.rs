pub(crate) mod auth;
mod config;
mod consensus;
mod device_registry;
mod extensions_bootstrap;
mod hex_api;
mod http_api;
mod internal_api;
mod peer_store;
mod rate_limit;
mod spv;

use std::sync::Arc;

use config::ValidatorConfig;
use consensus::create_app_state;
use device_registry::{load_registry, DEFAULT_REGISTRY_PATH};
use ed25519_dalek::{Signer, SigningKey};
use hexchain_p2p::block::HexBlock;
use hexchain_p2p::hex_consensus::{HexConsensus, HexProof};
use hexchain_p2p::types::{BlockHash, ConsensusParams, MmlParams};
use http_api::build_router;
use pot_o_extensions::tx::{hash_coinbase, CoinbaseTransaction};
use pot_o_extensions::{
    peer_network::RegistrationConfig, spawn_persist_ledger, LedgerEntry, LedgerSnapshot,
    DEFAULT_LEDGER_PATH,
};
use pot_o_mining::PotOConsensus;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Saved alongside ledger for canonical_tip_height + proof_traces path
#[derive(serde::Serialize, serde::Deserialize)]
struct ValidatorStateSnapshot {
    canonical_tip_height: u64,
}

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

    let state_path =
        std::env::var("STATE_PATH").unwrap_or_else(|_| "/blockstore/state.json".to_string());
    let proof_trace_path = std::env::var("PROOF_TRACE_PATH")
        .unwrap_or_else(|_| "/blockstore/proof_traces.json".to_string());

    // Load canonical_tip_height from state.json
    let loaded_canonical_tip = std::fs::read_to_string(&state_path)
        .ok()
        .and_then(|s| serde_json::from_str::<ValidatorStateSnapshot>(&s).ok())
        .map(|s| s.canonical_tip_height)
        .unwrap_or(0);

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
    let mempool_clone = extensions.mempool.clone();
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

    // Mempool persistence
    let mempool_path =
        std::env::var("MEMPOOL_PATH").unwrap_or_else(|_| "/blockstore/mempool.json".to_string());
    if let Some(ref mp) = mempool_clone {
        mp.set_path(&mempool_path);
        mp.load_from_file(&mempool_path);
        let ledger_for_reval = extensions.ledger.clone();
        let mp_for_reval = mp.clone();
        tokio::task::spawn_blocking(move || {
            mp_for_reval.revalidate(&ledger_for_reval);
        })
        .await
        .ok();
        tracing::info!(
            path = %mempool_path,
            pending = mp.len(),
            "Mempool loaded and revalidated"
        );
        let mp = mp.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                if mp.is_modified() {
                    let mp = mp.clone();
                    tokio::task::spawn_blocking(move || {
                        if let Err(e) = mp.save_to_file() {
                            tracing::warn!(error = %e, "Failed to persist mempool");
                        }
                    })
                    .await
                    .ok();
                }
            }
        });
        tracing::info!(path = %mempool_path, "Mempool persistence started");
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

    // Restore canonical tip height
    if loaded_canonical_tip > 0 {
        *state.canonical_tip_height.write().await = loaded_canonical_tip;
        tracing::info!(
            height = loaded_canonical_tip,
            "Canonical tip height restored"
        );
    }

    // Proof traces persistence
    state.proof_traces.set_path(&proof_trace_path);
    state.proof_traces.load_from_file();
    {
        let pt = state.proof_traces.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                let pt = pt.clone();
                tokio::task::spawn_blocking(move || {
                    let _ = pt.save_to_file();
                })
                .await
                .ok();
            }
        });
    }

    if let Some(ref bs) = block_store {
        let bs = bs.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                if bs.is_modified() {
                    let bs = bs.clone();
                    tokio::task::spawn_blocking(move || {
                        if let Err(e) = bs.save_to_file() {
                            tracing::error!("Failed to persist block store: {}", e);
                        } else {
                            bs.clear_modified();
                        }
                    })
                    .await
                    .ok();
                }
            }
        });
        tracing::info!("BlockStore background persistence started");
    }

    // Persist hex lattice every 10 seconds
    {
        let lattice = state.hex_consensus.store.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                let lattice = lattice.clone();
                tokio::task::spawn_blocking(move || {
                    if let Err(e) = lattice.save_to_file() {
                        tracing::warn!(error = %e, "Failed to persist hex lattice");
                    }
                })
                .await
                .ok();
            }
        });
        tracing::info!(path = %lattice_path, "Hex lattice background persistence started");
    }

    // ── Automated block producer ──────────────────────────────────────
    if tribechain_enabled {
        match load_miner_keypair(&cfg.tribechain_miner_keypair_path) {
            Ok(miner_keypair) => {
                let miner_address =
                    bs58::encode(miner_keypair.verifying_key().to_bytes()).into_string();
                tracing::info!(
                    miner_address = %miner_address,
                    "Block producer miner identity loaded"
                );

                let bp_state = Arc::clone(&state);
                let bp_mempool = mempool_clone.clone();
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
                    loop {
                        interval.tick().await;
                        let mp = match bp_mempool {
                            Some(ref mp) => mp,
                            None => continue,
                        };
                        let tip = *bp_state.canonical_tip_height.read().await;
                        let next_height = tip + 1;
                        let txs = if mp.is_empty() {
                            Vec::new()
                        } else {
                            mp.pending()
                        };
                        let slot = {
                            let stats = bp_state.stats.read().await;
                            stats.total_challenges_issued
                        };
                        let slot_hash = hex::encode(Sha256::digest(slot.to_le_bytes()));
                        let challenge = bp_state.hex_consensus.generate_challenge(slot, &slot_hash);
                        let block_reward =
                            pot_o_extensions::ledger::block_reward_at_height(next_height);
                        let proof_rewards = Vec::new();
                        let coinbase_hash = hash_coinbase(
                            next_height,
                            &miner_address,
                            block_reward,
                            &proof_rewards,
                        );
                        let signature = miner_keypair.sign(&coinbase_hash).to_bytes().to_vec();
                        let coinbase = CoinbaseTransaction {
                            tx_hash: coinbase_hash,
                            height: next_height,
                            miner_address: miner_address.clone(),
                            block_reward,
                            proof_rewards,
                            signature,
                        };
                        let coinbase_value = serde_json::to_value(&coinbase).unwrap_or_default();
                        let mut tx_values: Vec<serde_json::Value> = vec![coinbase_value];
                        for tx in &txs {
                            if let Ok(val) = serde_json::to_value(tx) {
                                tx_values.push(val);
                            }
                        }
                        let tx_merkle_root = compute_merkle_root(&tx_values);
                        let target = challenge.target;
                        let mut block = HexBlock {
                            parent_hash: challenge.neighbor_hashes[0],
                            height: next_height,
                            tx_merkle_root,
                            transactions: Some(tx_values),
                            miner_address: Some(miner_address.clone()),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                            nonce: 0,
                            coord: challenge.coord,
                            neighbor_hashes: challenge.neighbor_hashes,
                            tensor: hexchain_p2p::types::TensorMeta {
                                expected_capacity: 0,
                                actual_capacity: 0,
                                compression_num: 0,
                                compression_den: 1,
                            },
                        };
                        let mut found = false;
                        for nonce in 0..1_000_000u64 {
                            block.nonce = nonce;
                            let hash = block.pow_hash();
                            if hash <= target {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            tracing::debug!("Block producer could not find valid nonce, skipping");
                            continue;
                        }
                        let proof = HexProof {
                            challenge_id: challenge.id.clone(),
                            block,
                            miner_pubkey: miner_address.clone(),
                            timestamp_unix: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                            generation: challenge.generation,
                        };
                        match bp_state.hex_consensus.submit_block(&proof) {
                            Ok(depth) => {
                                let mut ledger = bp_state.extensions.ledger.write().await;
                                let mp_ref = bp_mempool.as_deref();
                                let bs_ref = bp_state.extensions.block_store.as_deref();
                                if let Err(e) = crate::consensus::accept_block(
                                    &proof.block,
                                    &mut ledger,
                                    mp_ref,
                                    bs_ref,
                                ) {
                                    tracing::warn!(error = %e, "Auto block producer: accept_block failed");
                                    continue;
                                }
                                *bp_state.canonical_tip_height.write().await += 1;
                                tracing::info!(
                                    height = proof.block.height,
                                    txs = txs.len(),
                                    depth = depth,
                                    "Auto block produced"
                                );

                                let block_hash = hex::encode(proof.block.pow_hash());
                                let _ = bp_state
                                    .extensions
                                    .messaging
                                    .broadcast(&pot_o_extensions::ValidatorMessage::NewBlock {
                                        height: proof.block.height,
                                        hash: block_hash,
                                        tx_count: txs.len(),
                                        timestamp: proof.timestamp_unix,
                                    })
                                    .await;

                                if let Ok(proof_json) = serde_json::to_value(&proof) {
                                    let network = bp_state.extensions.network.clone();
                                    tokio::spawn(async move {
                                        match network.broadcast_block(&proof_json).await {
                                            Ok(n) if n > 0 => {
                                                tracing::info!(
                                                    peers = n,
                                                    "Block broadcast to peers"
                                                );
                                            }
                                            Ok(_) => {}
                                            Err(e) => {
                                                tracing::warn!(error = %e, "Block broadcast to peers failed");
                                            }
                                        }
                                    });
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = ?e, "Auto block producer: submit_block failed");
                            }
                        }
                    }
                });
                tracing::info!("Automated block producer started");
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to load miner keypair — block producer disabled");
            }
        }
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
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                let _ = network.register_with_bootstrap(&reg_cfg).await;
                interval.tick().await;
            }
        });
        tracing::info!("Bootstrap registration task started ({} urls)", url_count);
    }

    let peer_store_path =
        std::env::var("PEER_STORE_PATH").unwrap_or_else(|_| "/blockstore/peers.json".to_string());

    let internal_state = internal_api::InternalApiState {
        node_id: cfg.node_id.clone(),
        peers: Arc::new(RwLock::new(Vec::new())),
        current_challenge: Arc::new(RwLock::new(None)),
        mempool: mempool_clone,
        ledger,
        tribechain_enabled,
        internal_mint_secret: std::env::var("INTERNAL_MINT_SECRET").ok(),
        peer_store: None,
    };

    let peer_store = Arc::new(peer_store::PeerStore::new(
        peer_store_path.clone(),
        internal_state.peers.clone(),
    ));
    let ps_for_load = peer_store.clone();
    tokio::task::spawn_blocking(move || {
        ps_for_load.load();
    })
    .await
    .ok();
    {
        let loaded = internal_state.peers.read().await.len();
        if loaded > 0 {
            tracing::info!(peers = loaded, path = %peer_store_path, "Peer list loaded from disk");
        }
    }
    peer_store.spawn_persist();
    tracing::info!(path = %peer_store_path, "Peer persistence started");

    let mut internal_state = internal_state;
    internal_state.peer_store = Some(peer_store.clone());

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
    let shutdown_state_path = state_path.clone();
    let shutdown_peer_store = peer_store.clone();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        let sigterm = async {
            #[cfg(unix)]
            {
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("failed to register SIGTERM handler")
                    .recv()
                    .await;
            }
            #[cfg(not(unix))]
            {
                std::future::pending::<()>().await;
            }
        };
        let sigint = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to register SIGINT handler");
        };
        tokio::select! {
            _ = sigterm => {},
            _ = sigint => {},
        }
        tracing::info!("Shutdown signal received — force-persisting state...");

        // Force-save ledger
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
            let nonces: Vec<(String, u64)> =
                l.nonces().iter().map(|(k, v)| (k.clone(), *v)).collect();
            let snapshot = LedgerSnapshot {
                entries,
                tx_history: l.tx_history().to_vec(),
                nonces,
                block_height: l.block_height(),
            };
            let json = match serde_json::to_string_pretty(&snapshot) {
                Ok(j) => j,
                Err(e) => {
                    tracing::error!("Failed to serialize ledger on shutdown: {}", e);
                    return;
                }
            };
            if let Err(e) = std::fs::write(&shutdown_ledger_path, json) {
                tracing::error!("Failed to persist ledger on shutdown: {}", e);
            } else {
                tracing::info!("Ledger saved to {}", shutdown_ledger_path);
            }
        }

        // Force-save canonical tip height + proof traces
        {
            let tip = *shutdown_state.canonical_tip_height.read().await;
            let snap = ValidatorStateSnapshot {
                canonical_tip_height: tip,
            };
            let json = match serde_json::to_string_pretty(&snap) {
                Ok(j) => j,
                Err(e) => {
                    tracing::error!("Failed to serialize validator state on shutdown: {}", e);
                    return;
                }
            };
            if let Err(e) = std::fs::write(&shutdown_state_path, json) {
                tracing::error!("Failed to persist validator state: {}", e);
            } else {
                tracing::info!("Validator state saved (tip={})", tip);
            }
        }
        shutdown_state.proof_traces.save_to_file().ok();

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

        // Force-save mempool
        if let Some(ref mp) = shutdown_state.extensions.mempool {
            if mp.is_modified() {
                if let Err(e) = mp.save_to_file() {
                    tracing::warn!("Failed to persist mempool on shutdown: {}", e);
                } else {
                    tracing::info!("Mempool saved");
                }
            }
        }

        if let Err(e) = shutdown_peer_store.save().await {
            tracing::warn!("Failed to persist peers on shutdown: {}", e);
        } else {
            tracing::info!("Peer list saved");
        }

        tracing::info!("State persistence complete — exiting.");
    })
    .await
    .unwrap();
}

fn compute_merkle_root(txs: &[serde_json::Value]) -> BlockHash {
    let leaves: Vec<BlockHash> = txs
        .iter()
        .map(|tx| {
            let data = serde_json::to_string(tx).unwrap_or_default();
            let hash = Sha256::digest(data.as_bytes());
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&hash);
            arr
        })
        .collect();
    if leaves.is_empty() {
        return [0u8; 32];
    }
    let mut level = leaves;
    while level.len() > 1 {
        let mut next = Vec::new();
        for chunk in level.chunks(2) {
            let mut hasher = Sha256::new();
            hasher.update(chunk[0]);
            hasher.update(if chunk.len() > 1 {
                &chunk[1]
            } else {
                &chunk[0]
            });
            let hash = hasher.finalize();
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&hash);
            next.push(arr);
        }
        level = next;
    }
    level[0]
}

fn load_miner_keypair(path: &str) -> Result<SigningKey, String> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        format!(
            "Miner keypair file not found at '{}': {}. \
             Generate one with: solana-keygen new --outfile {}",
            path, e, path
        )
    })?;
    let bytes: Vec<u8> = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid keypair JSON at '{}': {}", path, e))?;
    if bytes.len() != 64 {
        return Err(format!(
            "Keypair file must contain 64 bytes, got {}",
            bytes.len()
        ));
    }
    let signing_key = SigningKey::from_bytes(
        &bytes[..32]
            .try_into()
            .map_err(|_| "Invalid secret key length".to_string())?,
    );
    let public = signing_key.verifying_key();
    if bytes[32..] != public.to_bytes() {
        return Err("Keypair file has mismatched public key".to_string());
    }
    tracing::info!(path = %path, "Miner keypair loaded from file");
    Ok(signing_key)
}
