# pot-o-validator

[![crates.io](https://img.shields.io/crates/v/pot-o-validator.svg)](https://crates.io/crates/pot-o-validator)
[![GitHub release](https://img.shields.io/github/v/release/TribeWarez/pot-o-validator)](https://github.com/TribeWarez/pot-o-validator/releases/latest)
[![docs.rs](https://img.shields.io/docsrs/pot-o-validator)](https://docs.rs/pot-o-validator)
[![CI](https://img.shields.io/github/actions/workflow/status/TribeWarez/pot-o-validator/pot-o-validator.yml?branch=main)](https://github.com/TribeWarez/pot-o-validator/actions)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](LICENSE)

PoT-O (Proof of Tensor Optimizations) blockchain validator — HTTP API, HexChain lattice consensus, and TribeChain multi-token economy.

- **Crate:** [crates.io/crates/pot-o-validator](https://crates.io/crates/pot-o-validator)
- **Docs:** [docs.rs/pot-o-validator](https://docs.rs/pot-o-validator)
- **Repository:** [github.com/TribeWarez/pot-o-validator](https://github.com/TribeWarez/pot-o-validator)

## Architecture

PoT-O consensus validates tensor optimization proofs on a HexChain lattice structure with difficulty adjustment (30s target block time). The network supports 7 tokens with enforced supply caps and authenticated P2P transport (ed25519 v2).

## Crates

| Crate | Description |
|-------|-------------|
| **pot-o-validator** | HTTP API, config, consensus engine, device registry, extensions bootstrap |
| **pot-o-core** | Block/transaction types, errors, constants, token definitions |
| **ai3-lib** | Tensor engine, ESP-compatible mining operations |
| **pot-o-mining** | Challenge generation, MML/neural path validation, PoT-O consensus |
| **pot-o-extensions** | DeFi, pool strategy, device protocol, chain bridge, peer network, security |
| **hexchain-p2p** | Authenticated P2P transport layer with ed25519 node identities |

## Binary Targets

| Binary | Description |
|--------|-------------|
| `pot-o-validator` | Main validator server (HTTP API + consensus) |
| `pot-o-mml-calibrate` | MML calibration tool for mining parameters |
| `pot-o-timing` | Timing benchmarks for proof validation |
| `pot-o-golden-proofs` | Golden proof generation for test vectors |
| `tribechain-genesis` | Genesis block generator for TribeChain |

## Token Economics

| Token | Supply Cap |
|-------|-----------|
| TRIBE | 1,000,000,000,000 |
| PTtC | 21,000,000,000,000 |
| NMTC | 100,000,000,000 |
| AI3 | 1,000,000,000,000 |
| STOMP | Enforced cap |
| AUM | Enforced cap |
| RAVECOIN | Enforced cap |

All caps enforced via `checked_add` overflow protection and `try_issue()` on mining rewards.

## Quick Start

**Docker:**
```bash
docker build -t pot-o-validator .
docker run -p 8900:8900 pot-o-validator
```

**Local development:**
```bash
cargo build --release
cargo run --release --bin pot-o-validator
```

## Configuration

Config loads from `/config/default.toml`, then `config/default.toml`, then built-in defaults. Key env vars:

| Variable | Description | Default |
|----------|-------------|---------|
| `PORT` | HTTP API port | 8900 |
| `POT_O_DIFFICULTY` | Mining difficulty | — |
| `PEER_NETWORK_MODE` | `local_only` or `vpn_mesh` | `local_only` |
| `BOOTSTRAP_URLS` | Comma-separated peer discovery URLs | — |
| `POOL_STRATEGY` | Mining pool strategy | — |
| `ENABLE_MDNS` | Enable mDNS peer discovery | — |

## API Endpoints

- `GET /status` — Node status and block height
- `POST /challenge` — Request mining challenge
- `POST /submit` — Submit proof
- `POST /tx` — Submit transaction
- `GET /spv/proof` — SPV Merkle proof for light clients
- `GET /peers` — Connected peer list
- `DELETE /marketplace/order` — Cancel marketplace order (authenticated)

## Security Features

- Per-IP rate limiting on challenge/submit/tx endpoints
- ed25519 v2 authenticated P2P transport (CVE-2024-31167 mitigated)
- Shared-secret auth on `/internal/mint`
- Docker non-root user (UID 1000)
- Supply cap enforcement on all 7 token types
- Sanitized error messages to prevent information leakage
- Helmet + rate limiting on wallet service
- Address validation on faucet

## Testing

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --release
```

Per-crate: `cargo test -p <crate> --release`

## Versioning

Releases follow semantic versioning. Tag format: `pot-o-validator-v*.*.*`. CI publishes to crates.io in dependency order: `pot-o-core` → `ai3-lib` → `pot-o-mining` → `pot-o-extensions` → `pot-o-validator`.

## License

[GPL-3.0](LICENSE) — TribeWarez.
