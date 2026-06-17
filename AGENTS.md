# AGENTS.md — PoT-O Validator

## Workspace

Cargo workspace with 6 crates at root: `core/`, `ai3-lib/`, `mining/`, `extensions/`, `hexchain-p2p/`, plus the root crate `pot-o-validator`.

Local path deps via `[patch.crates-io]` in root `Cargo.toml` — do not use `cargo publish` from sub-crates individually without understanding the publish workflow.

## CI pipeline order (exact commands)

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo build --release
cargo test --release
```

Run these in order before pushing. No pre-commit hook or formatter config exists — uses rustfmt defaults.

## Binary targets (4)

- `pot-o-validator` — main server (`src/main.rs`)
- `pot-o-mml-calibrate` — MML calibration tool (`src/bin/pot_o_mml_calibrate.rs`)
- `pot-o-timing` — timing benchmark (`src/bin/pot_o_timing.rs`)
- `pot-o-golden-proofs` — golden proof generation (`src/bin/pot_o_golden_proofs.rs`)

Run with: `cargo run --release --bin <name>`

## Testing

- **Unit tests** per crate: `cargo test -p <crate> --release`
- **Integration tests**: `cargo test --release` (from root, runs everything)
- Integration tests live in `tests/` at root; each crate also has its own `tests/`
- Test fixtures: `tests/fixtures/pot_o_mml_vectors.json`
- Use `#[tokio::test]` for async integration tests; `tempfile::TempDir` for temp files

## Config

- Default: `config/default.toml`
- Runtime load: tries `/config/default.toml` first, then `config/default.toml`, then built-in defaults
- Most fields overridable via env vars — see `src/config.rs:172-249` for the full list. Key ones:
  `SOLANA_RPC_URL`, `POT_O_DIFFICULTY`, `PORT`, `PEER_NETWORK_MODE`, `POOL_STRATEGY`,
  `CHAIN_BRIDGE`, `RELAYER_KEYPAIR_PATH`, `ENABLE_MDNS`
- HTTP port default: **8900**

## Release & publish

- Tags: `pot-o-validator-v*` or `pot-o-extensions-v*` trigger publish to crates.io
- Publish order (enforced in CI): `pot-o-core` → `ai3-lib` → `pot-o-mining` → `pot-o-extensions` → `pot-o-validator`
- Each waits for crates.io indexing (up to 5 min) before next publish
- The publish workflow uses `--allow-dirty` for the final crate
- `[profile.release]`: opt-level 3, LTO, 1 codegen unit, panic=abort

## Docker

- Dockerfile builds only `pot-o-validator` binary (not the 3 auxiliary bins)
- `CARGO_BUILD_JOBS=2` to limit peak disk usage (Solana deps are large)
- Exposes port **8900**

## Notable structure

- `src/lib.rs` re-exports all workspace crates (`pub use ai3_lib::*;` etc.)
- ESP32/8266 firmware lives in `firmware/esp-pot-o-miner/` (PlatformIO, not Cargo)
- Solana keypairs in `keys/relayer.json` (submissions) and `keys/miner.json` (mining)
- Device registry persisted to `device_registry.json` at runtime
- No `rust-toolchain.toml`, no `.cargo/config.toml`, no Makefile

## Important env vars for production

- `SOLANA_RPC_URL` — Solana RPC endpoint
- `RELAYER_KEYPAIR_PATH` — path to Solana keypair for on-chain submissions
- `BOOTSTRAP_URLS` — comma-separated peer discovery URLs
- `PEER_NETWORK_MODE` — `"local_only"` (default in code) vs `"vpn_mesh"` (default in config file)
