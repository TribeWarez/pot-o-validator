//! Builds the extension registry from validator config (chain, pool, network, P2P, auth).

use pot_o_extensions::{
    ExtensionRegistry, HexchainAuthority, HexchainNetwork, NativeDevice, SoloStrategy,
};

use crate::config::ValidatorConfig;

/// Builds an [`ExtensionRegistry`] with local defaults using config (Solana RPC, program ID, relayer keypair, auto-register).
///
/// When `p2p_listen_port > 0`, replaces the local-only network and Ed25519
/// authority with hexchain geometric P2P and lattice-aware security.
pub fn build_extension_registry(cfg: &ValidatorConfig) -> ExtensionRegistry {
    if cfg.p2p_listen_port > 0 {
        build_hexchain_registry(cfg)
    } else {
        build_local_registry(cfg)
    }
}

/// Build a registry for single-node local operation (default).
fn build_local_registry(cfg: &ValidatorConfig) -> ExtensionRegistry {
    use pot_o_extensions::{Ed25519Authority, LocalOnlyNetwork, SolanaBridge};
    ExtensionRegistry {
        device: Box::new(NativeDevice::new()),
        network: Box::new(LocalOnlyNetwork::new()),
        pool: Box::new(SoloStrategy),
        chain: Box::new(SolanaBridge::new(
            cfg.solana_rpc_url.clone(),
            cfg.pot_program_id.clone(),
            cfg.relayer_keypair_path.clone(),
            cfg.auto_register_miners,
        )),
        auth: Box::new(Ed25519Authority),
    }
}

/// Build a registry with hexchain geometric P2P and lattice-aware security.
fn build_hexchain_registry(cfg: &ValidatorConfig) -> ExtensionRegistry {
    use pot_o_extensions::SolanaBridge;

    // Generate or load an ed25519 keypair for P2P identity
    let keypair = load_or_generate_keypair(cfg);

    let bootstrap_nodes: Vec<String> = if cfg.p2p_bootstrap_nodes.is_empty() {
        vec![]
    } else {
        cfg.p2p_bootstrap_nodes
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    let network = HexchainNetwork::new(
        cfg.node_id.clone(),
        keypair,
        cfg.p2p_listen_port,
        bootstrap_nodes,
    );

    // Re-derive keypair for authority (same keypair used for network identity)
    let auth_keypair = load_or_generate_keypair(cfg);

    ExtensionRegistry {
        device: Box::new(NativeDevice::new()),
        network: Box::new(network),
        pool: Box::new(SoloStrategy),
        chain: Box::new(SolanaBridge::new(
            cfg.solana_rpc_url.clone(),
            cfg.pot_program_id.clone(),
            cfg.relayer_keypair_path.clone(),
            cfg.auto_register_miners,
        )),
        auth: Box::new(HexchainAuthority::new(auth_keypair)),
    }
}

/// Load keypair from file or generate a fresh one.
///
/// Tries `RELAYER_KEYPAIR_PATH` first (Solana-compatible JSON),
/// then falls back to generating a random keypair (for ephemeral nodes).
fn load_or_generate_keypair(cfg: &ValidatorConfig) -> ed25519_dalek::Keypair {
    // Try loading from relayer keypair path (Solana JSON format)
    if let Ok(data) = std::fs::read_to_string(&cfg.relayer_keypair_path) {
        if let Ok(secret) = serde_json::from_str::<Vec<u8>>(&data) {
            if secret.len() == 64 {
                if let Ok(kp) = ed25519_dalek::Keypair::from_bytes(&secret) {
                    return kp;
                }
            }
        }
    }

    // Generate a fresh keypair for ephemeral nodes (no persisted identity)
    use ed25519_dalek::SecretKey;
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut seed = [0u8; 32];
    rng.fill(&mut seed);
    let secret = SecretKey::from_bytes(&seed).expect("valid 32-byte secret key");
    let public = ed25519_dalek::PublicKey::from(&secret);
    ed25519_dalek::Keypair { secret, public }
}
